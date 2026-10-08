/**
 * ************************************************************************************************
 * @brief  LLVM ModulePass that lets front-ends other than clang opt in to ASPIS annotations.
 *
 * Converts any global symbol marked with `#[unsafe(link_section = "aspis_<annotation>")]` to
 * the corresponding ASPIS `__attribute__((annotate("<annotation>")))`. Multiple annotations can
 * be given as a comma-separated list, e.g. `link_section = "aspis_to_harden,aspis_exclude"`.
 *
 * ************************************************************************************************
 */
#include "llvm/ADT/SmallVector.h"
#include "llvm/IR/Constants.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/GlobalObject.h"
#include "llvm/IR/GlobalVariable.h"
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

  // The section may hold a comma-separated list of markers, e.g.
  // "aspis_to_harden, aspis_exclude". It is only treated as ASPIS annotations if every entry
  // is a known marker; anything else is left untouched as a regular link section.
  std::vector<std::string> getASPISAnnotations(GlobalObject &GO)
  {
    if (!GO.hasSection())
    {
      return {};
    }

    SmallVector<StringRef, 4> Entries;
    GO.getSection().split(Entries, ',');

    std::vector<std::string> Annotations;
    for (StringRef Entry : Entries)
    {
      std::optional<StringRef> Annotation = getASPISAnnotation(Entry.trim());
      if (!Annotation.has_value())
      {
        return {};
      }
      Annotations.emplace_back(Annotation.value());
    }
    return Annotations;
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
      for (std::string &Annotation : getASPISAnnotations(GO))
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

    if (ToAnnotate.empty())
    {
      return PreservedAnalyses::all();
    }

    for (auto &[GV, Annotation] : ToAnnotate)
    {
      GV->setSection("");
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
