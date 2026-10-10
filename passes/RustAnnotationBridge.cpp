/**
 * ************************************************************************************************
 * @brief  LLVM ModulePass that lets front-ends other than clang opt in to ASPIS annotations.
 *
 * Converts any global symbol marked with `#[unsafe(link_section = "aspis_<annotation>")]` to
 * the corresponding ASPIS `__attribute__((annotate("<annotation>")))`. Multiple annotations can
 * be given as a comma-separated list, e.g. `link_section = "aspis_to_harden,aspis_exclude"`, which
 * may also contain a regular section (e.g. `".data,aspis_to_harden"`) that is kept on the symbol.
 *
 * For programs using Rust's std, it also marks the rustc-generated startup code (the C `main`,
 * `std::rt::lang_start` and the closures/shims only it uses) as `exclude`. That code is entered
 * from libstd through function pointers, which the control-flow checks cannot follow; excluding
 * it makes the user's `main` the first hardened function to run, as it is for C programs.
 *
 * ************************************************************************************************
 */
#include "llvm/ADT/SetVector.h"
#include "llvm/ADT/SmallPtrSet.h"
#include "llvm/ADT/SmallVector.h"
#include "llvm/ADT/StringExtras.h"
#include "llvm/IR/Constants.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/GlobalObject.h"
#include "llvm/IR/GlobalVariable.h"
#include "llvm/IR/InstIterator.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Pass.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include <optional>
#include <string>
#include <utility>
#include <vector>

using namespace llvm;

#define DEBUG_TYPE "aspis-rust-annotation-bridge"

namespace
{
  const StringRef SectionPrefix = "aspis_";

  std::optional<StringRef> getASPISAnnotation(StringRef Entry)
  {
    if (Entry == "aspis_to_harden")
    {
      return {"to_harden"};
    }
    else if (Entry == "aspis_to_duplicate")
    {
      return {"to_duplicate"};
    }
    else if (Entry == "aspis_exclude")
    {
      return {"exclude"};
    }
    else
    {
      return std::nullopt;
    }
  }

  // The section may hold a comma-separated list mixing ASPIS markers with regular sections,
  // e.g. ".data, aspis_to_harden". The ASPIS markers are stripped from the section and returned
  // as annotations; the remaining entries are kept as the symbol's section.
  std::vector<std::string> extractASPISAnnotations(GlobalObject &GO)
  {
    if (!GO.hasSection())
    {
      return {};
    }

    SmallVector<StringRef, 4> Entries;
    GO.getSection().split(Entries, ',');

    std::vector<std::string> Annotations;
    SmallVector<StringRef, 4> Remaining;
    for (StringRef Entry : Entries)
    {
      Entry = Entry.trim();
      if (std::optional<StringRef> Annotation = getASPISAnnotation(Entry))
      {
        Annotations.emplace_back(Annotation.value());
      }
      else
      {
        Remaining.push_back(Entry);
      }
    }

    if (!Annotations.empty())
    {
      GO.setSection(join(Remaining, ","));
    }
    return Annotations;
  }
  // Whether every use of V, looking through constant expressions and initializers, lies in
  // Glue: in an instruction of one of its functions, or in the initializer of one of its globals.
  bool isOnlyUsedBy(const Value *V, const SetVector<GlobalObject *> &Glue)
  {
    for (const User *U : V->users())
    {
      if (const auto *I = dyn_cast<Instruction>(U))
      {
        if (!Glue.contains(const_cast<Function *>(I->getFunction())))
        {
          return false;
        }
      }
      else if (const auto *GO = dyn_cast<GlobalObject>(U))
      {
        if (!Glue.contains(const_cast<GlobalObject *>(GO)))
        {
          return false;
        }
      }
      else if (!isa<Constant>(U) || !isOnlyUsedBy(U, Glue))
      {
        return false;
      }
    }
    return true;
  }

  // Collects the startup code rustc emits for a `fn main()` using std: a C `main` that calls
  // `std::rt::lang_start(<user main>, ...)`, plus every non-exported function or global that is
  // only reachable from that code (closures, vtables, FnOnce shims, Termination::report, ...).
  // The user's main is never included. Returns an empty set for #![no_main] and C programs.
  SetVector<GlobalObject *> findRustStartupGlue(Module &Md)
  {
    SetVector<GlobalObject *> Glue;

    Function *CMain = Md.getFunction("main");
    if (CMain == nullptr || CMain->isDeclaration())
    {
      return Glue;
    }

    Function *UserMain = nullptr;
    for (Instruction &I : instructions(*CMain))
    {
      auto *Call = dyn_cast<CallBase>(&I);
      Function *Callee = Call ? Call->getCalledFunction() : nullptr;
      if (Callee == nullptr || Callee->isDeclaration() || !Callee->getName().contains("lang_start") ||
          Call->arg_empty())
      {
        continue;
      }
      UserMain = dyn_cast<Function>(Call->getArgOperand(0)->stripPointerCasts());
      if (UserMain != nullptr)
      {
        Glue.insert(CMain);
        Glue.insert(Callee);
        break;
      }
    }

    if (UserMain == nullptr)
    {
      return Glue;
    }

    bool Changed = true;
    while (Changed)
    {
      Changed = false;
      for (GlobalObject &GO : Md.global_objects())
      {
        if (&GO == UserMain || Glue.contains(&GO) || GO.isDeclaration() ||
            !(GO.hasLocalLinkage() || GO.hasHiddenVisibility()) || GO.use_empty())
        {
          continue;
        }
        if (isOnlyUsedBy(&GO, Glue))
        {
          Glue.insert(&GO);
          Changed = true;
        }
      }
    }

    return Glue;
  }
} // namespace

class RustAnnotationBridge : public PassInfoMixin<RustAnnotationBridge>
{
public:
  PreservedAnalyses run(Module &Md, ModuleAnalysisManager &)
  {
    std::vector<std::pair<GlobalObject *, std::string>> ToAnnotate;

    auto Collect = [&ToAnnotate](GlobalObject &GO)
    {
      for (std::string &Annotation : extractASPISAnnotations(GO))
      {
        ToAnnotate.emplace_back(&GO, std::move(Annotation));
      }
    };

    for (GlobalVariable &GV : Md.globals())
    {
      Collect(GV);
    }

    for (Function &Fn : Md)
    {
      Collect(Fn);
    }

    for (GlobalObject *GO : findRustStartupGlue(Md))
    {
      if (auto *Fn = dyn_cast<Function>(GO))
      {
        ToAnnotate.emplace_back(Fn, "exclude");
      }
    }

    if (ToAnnotate.empty())
    {
      return PreservedAnalyses::all();
    }

    addAnnotations(Md, ToAnnotate);

    return PreservedAnalyses::none();
  }

  static bool isRequired() { return true; }

private:
  static void addAnnotations(Module &Md,
                             ArrayRef<std::pair<GlobalObject *, std::string>> ToAnnotate)
  {
    LLVMContext &Ctx = Md.getContext();
    auto *PtrTy = PointerType::getUnqual(Ctx);
    auto *EntryTy = StructType::get(Ctx, {PtrTy, PtrTy});
    std::vector<Constant *> Entries;

    // Preserve any annotations clang already emitted
    if (GlobalVariable *Existing = Md.getGlobalVariable("llvm.global.annotations"))
    {
      if (auto *CA = dyn_cast<ConstantArray>(Existing->getInitializer()))
      {
        for (Value *Op : CA->operands())
        {
          Entries.push_back(cast<Constant>(Op));
        }
      }
      Existing->eraseFromParent();
    }

    for (auto &[GV, Annotation] : ToAnnotate)
    {
      Constant *Str = ConstantDataArray::getString(Ctx, Annotation, /*AddNull=*/true);
      auto *StrGV = new GlobalVariable(Md, Str->getType(), /*isConstant=*/true,
                                       GlobalValue::PrivateLinkage, Str,
                                       GV->getName() + ".aspis_annotation");
      StrGV->setSection("llvm.metadata");
      StrGV->setUnnamedAddr(GlobalValue::UnnamedAddr::Global);

      Entries.push_back(ConstantStruct::get(EntryTy, {GV, StrGV}));
    }

    auto *ArrTy = ArrayType::get(EntryTy, Entries.size());
    auto *NewGlobal = new GlobalVariable(
        Md, ArrTy, /*isConstant=*/false, GlobalValue::AppendingLinkage,
        ConstantArray::get(ArrTy, Entries), "llvm.global.annotations");
    NewGlobal->setSection("llvm.metadata");
  }
};

llvm::PassPluginLibraryInfo getRustAnnotationBridgePluginInfo()
{
  return {LLVM_PLUGIN_API_VERSION, "aspis-rust-annotation-bridge", LLVM_VERSION_STRING,
          [](PassBuilder &PB)
          {
            PB.registerPipelineParsingCallback(
                [](StringRef Name, ModulePassManager &MPM,
                   ArrayRef<PassBuilder::PipelineElement>)
                {
                  if (Name == "aspis-rust-annotation-bridge")
                  {
                    MPM.addPass(RustAnnotationBridge());
                    return true;
                  }
                  return false;
                });
          }};
}

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo()
{
  return getRustAnnotationBridgePluginInfo();
}
