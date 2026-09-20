/**
 * ************************************************************************************************
 * @brief  LLVM ModulePass that lets front-ends other than clang opt in to ASPIS annotations.
 *
 * Converts any global symbol marked with `#[unsafe(link_section = "aspis_<annotation>")]` to
 * the corresponding ASPIS `__attribute__((annotate("<annotation>")))`.
 *
 * ************************************************************************************************
 */
#include "llvm/IR/Constants.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/GlobalObject.h"
#include "llvm/IR/GlobalVariable.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Pass.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include <utility>
#include <vector>

using namespace llvm;

#define DEBUG_TYPE "aspis-rust-annotation-bridge"

namespace
{
  const StringRef SectionPrefix = "aspis_";

  std::optional<std::string> getASPISAnnotation(GlobalObject &GO)
  {
    if (!GO.hasSection())
    {
      return std::nullopt;
    }

    StringRef Section = GO.getSection();
    if (Section == "aspis_to_harden")
    {
      return {"to_harden"};
    }
    else if (Section == "aspis_to_duplicate")
    {
      return {"to_duplicate"};
    }
    else if (Section == "aspis_exclude")
    {
      return {"exclude"};
    }
    else
    {
      return std::nullopt;
    }
  }
} // namespace

class RustAnnotationBridge : public PassInfoMixin<RustAnnotationBridge>
{
public:
  PreservedAnalyses run(Module &Md, ModuleAnalysisManager &)
  {
    std::vector<std::pair<GlobalObject *, StringRef>> ToAnnotate;

    for (GlobalVariable &GV : Md.globals())
    {
      std::optional<std::string> Annotation = getASPISAnnotation(GV);
      if (Annotation.has_value())
      {
        ToAnnotate.emplace_back(&GV, Annotation.value());
      }
    }

    for (Function &Fn : Md)
    {
      std::optional<std::string> Annotation = getASPISAnnotation(Fn);
      if (Annotation.has_value())
      {
        ToAnnotate.emplace_back(&Fn, Annotation.value());
      }
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
                             ArrayRef<std::pair<GlobalObject *, StringRef>> ToAnnotate)
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
