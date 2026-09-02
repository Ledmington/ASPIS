/**
 * ************************************************************************************************
 * @brief  LLVM ModulePass that lets front-ends other than clang opt in to ASPIS annotations.
 *
 *         ASPIS reads its per-symbol directives (`to_duplicate`, `exclude`, ...) from
 *         `@llvm.global.annotations`, which clang populates from the source-level
 *         `__attribute__((annotate(...)))`. rustc has no equivalent attribute, so Rust code
 *         instead marks a global with `#[unsafe(link_section = "aspis_<annotation>")]`. This pass
 *         must run before every other ASPIS pass: it looks for globals placed in a section named
 *         "aspis_<annotation>", removes that (otherwise meaningless, and never meant to reach the
 *         linker) section, and appends an equivalent entry to `@llvm.global.annotations` so the
 *         rest of the pipeline sees it exactly as it would a clang-emitted annotation.
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

#define DEBUG_TYPE "rust-annotation-bridge"

namespace {
const StringRef SectionPrefix = "aspis_";

// A global qualifies if it sits in a section named "aspis_<annotation>"; the annotation
// is whatever follows the prefix, so any annotation ASPIS understands works without
// this pass needing to know its name.
bool getRustAnnotation(GlobalObject &GO, StringRef &Annotation) {
  if (!GO.hasSection())
    return false;

  StringRef Section = GO.getSection();
  if (!Section.starts_with(SectionPrefix))
    return false;

  Annotation = Section.substr(SectionPrefix.size());
  return !Annotation.empty();
}
} // namespace

class RustAnnotationBridge : public PassInfoMixin<RustAnnotationBridge> {
public:
  PreservedAnalyses run(Module &Md, ModuleAnalysisManager &) {
    std::vector<std::pair<GlobalObject *, StringRef>> ToAnnotate;

    for (GlobalVariable &GV : Md.globals()) {
      StringRef Annotation;
      if (getRustAnnotation(GV, Annotation))
        ToAnnotate.emplace_back(&GV, Annotation);
    }
    for (Function &Fn : Md) {
      StringRef Annotation;
      if (getRustAnnotation(Fn, Annotation))
        ToAnnotate.emplace_back(&Fn, Annotation);
    }

    if (ToAnnotate.empty())
      return PreservedAnalyses::all();

    for (auto &[GV, Annotation] : ToAnnotate)
      GV->setSection("");

    addAnnotations(Md, ToAnnotate);

    return PreservedAnalyses::none();
  }

  static bool isRequired() { return true; }

private:
  static void addAnnotations(Module &Md,
                              ArrayRef<std::pair<GlobalObject *, StringRef>> ToAnnotate) {
    LLVMContext &Ctx = Md.getContext();
    auto *PtrTy = PointerType::getUnqual(Ctx);
    // {target, annotation string}: the same two leading fields clang emits, and the only
    // ones Utils::getFuncAnnotations() reads.
    auto *EntryTy = StructType::get(Ctx, {PtrTy, PtrTy});

    std::vector<Constant *> Entries;

    // Preserve any annotations clang already emitted, e.g. in a module linked together
    // from both C and Rust translation units.
    if (GlobalVariable *Existing = Md.getGlobalVariable("llvm.global.annotations")) {
      if (auto *CA = dyn_cast<ConstantArray>(Existing->getInitializer()))
        for (Value *Op : CA->operands())
          Entries.push_back(cast<Constant>(Op));
      Existing->eraseFromParent();
    }

    for (auto &[GV, Annotation] : ToAnnotate) {
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

llvm::PassPluginLibraryInfo getRustAnnotationBridgePluginInfo() {
  return {LLVM_PLUGIN_API_VERSION, "rust-annotation-bridge", LLVM_VERSION_STRING,
          [](PassBuilder &PB) {
            PB.registerPipelineParsingCallback(
                [](StringRef Name, ModulePassManager &MPM,
                   ArrayRef<PassBuilder::PipelineElement>) {
                  if (Name == "rust-annotation-bridge") {
                    MPM.addPass(RustAnnotationBridge());
                    return true;
                  }
                  return false;
                });
          }};
}

// This is the core interface for pass plugins. It guarantees that 'opt' will
// be able to recognize RustAnnotationBridge when added to the pass pipeline on the
// command line, i.e. via '-passes=rust-annotation-bridge'
extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
  return getRustAnnotationBridgePluginInfo();
}
