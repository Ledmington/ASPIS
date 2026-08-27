/**
 * ************************************************************************************************
 * @brief  LLVM ModulePass that bridges ASPIS's Rust front-end support (the
 *         aspis-annotations crate) to its existing annotation-parsing
 *         infrastructure.
 *
 *         rustc has no equivalent of clang's __attribute__((annotate(...))).
 *         Data is marked instead by giving a `static` the type
 *         ToHarden<X>/ToDuplicate<X>/Exclude<X> (see
 *         rust-annotations/src/lib.rs): rustc's own codegen already lowers
 *         these single-field wrapper structs down to X's raw storage layout
 *         regardless of repr, so there's no unwrapping to do here - but that
 *         also means the wrapper's identity leaves no trace in the plain
 *         LLVM IR type system either. The only place "this static's Rust
 *         type was ToHarden<i32>" survives at all is DWARF debug info
 *         (DIGlobalVariable -> DIType -> name "ToHarden<i32>"), which is
 *         therefore a hard requirement: the module must be compiled with
 *         debug info (-g) for this pass to find anything to convert.
 *
 *         Functions can't be wrapped in a generic type at all, so `main`
 *         and friends are marked directly with #[link_section] instead
 *         (still required there - see rust-annotations/src/lib.rs).
 *
 *         Either way, this pass converts what it finds into the exact
 *         llvm.global.annotations shape clang emits for the C attribute, so
 *         every other pass's Utils::getFuncAnnotations() call picks it up
 *         with zero changes.
 *
 *         Must run before any pass that reads annotations, i.e. first in
 *         the pipeline, before lower-switch/func-ret-to-ref.
 * ************************************************************************************************
*/
#include "ASPIS.h"
#include "llvm/IR/Constants.h"
#include "llvm/IR/DebugInfo.h"
#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/GlobalVariable.h"
#include "llvm/IR/Module.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"

using namespace llvm;

#define DEBUG_TYPE "rust-annotation-bridge"

namespace {

// Keep in sync with rust-annotations/src/lib.rs.
const std::pair<StringRef, StringRef> WrapperTypeToAnnotation[] = {
    {"ToHarden<", "to_harden"},
    {"ToDuplicate<", "to_duplicate"},
    {"Exclude<", "exclude"},
};

const std::pair<StringRef, StringRef> SectionToAnnotation[] = {
    {"aspis_to_harden", "to_harden"},
    {"aspis_to_duplicate", "to_duplicate"},
    {"aspis_exclude", "exclude"},
};

StringRef annotationForWrapperTypeName(StringRef TypeName) {
    for (const auto &Entry : WrapperTypeToAnnotation) {
        if (TypeName.starts_with(Entry.first)) {
            return Entry.second;
        }
    }
    return StringRef();
}

StringRef annotationForSection(StringRef Section) {
    for (const auto &Entry : SectionToAnnotation) {
        if (Section == Entry.first) {
            return Entry.second;
        }
    }
    return StringRef();
}

// Recovers the Rust-level type name of a global's declared type from its
// debug info, e.g. "ToHarden<i32>" - information that no longer exists
// anywhere in the global's actual LLVM IR type (see file comment above).
// Returns an empty StringRef if the module has no debug info for GV.
StringRef debugTypeName(GlobalVariable &GV) {
    SmallVector<DIGlobalVariableExpression *, 1> GVEs;
    GV.getDebugInfo(GVEs);
    for (DIGlobalVariableExpression *GVE : GVEs) {
        if (DIGlobalVariable *DGV = GVE->getVariable()) {
            if (DIType *Ty = DGV->getType()) {
                return Ty->getName();
            }
        }
    }
    return StringRef();
}

// Creates a private, null-terminated string constant global, matching the
// shape clang emits for annotation strings (a ConstantDataArray
// initializer, read directly by Utils::getFuncAnnotations).
GlobalVariable *createStringConstant(Module &Md, StringRef Str, const Twine &Name) {
    Constant *StrConstant = ConstantDataArray::getString(Md.getContext(), Str, /*AddNull=*/true);
    auto *GV = new GlobalVariable(Md, StrConstant->getType(), /*isConstant=*/true,
                                   GlobalValue::PrivateLinkage, StrConstant, Name);
    GV->setUnnamedAddr(GlobalValue::UnnamedAddr::Global);
    return GV;
}

} // namespace

PreservedAnalyses RustAnnotationBridge::run(Module &Md, ModuleAnalysisManager &) {
    std::vector<std::pair<GlobalValue *, StringRef>> Found;

    for (GlobalVariable &GV : Md.globals()) {
        StringRef Annotation = annotationForWrapperTypeName(debugTypeName(GV));
        if (!Annotation.empty()) {
            Found.push_back({&GV, Annotation});
        }
    }

    // Functions are GlobalValues too and can carry #[link_section] just
    // like statics - used e.g. to mark Rust's runtime-entry `main` as
    // `exclude`, since it's invoked indirectly through std::rt::lang_start
    // rather than by a direct call ASPIS's pass could rewrite, and must
    // keep its original signature untouched.
    for (Function &Fn : Md.functions()) {
        if (!Fn.hasSection()) {
            continue;
        }
        StringRef Annotation = annotationForSection(Fn.getSection());
        if (!Annotation.empty()) {
            Found.push_back({&Fn, Annotation});
        }
    }

    if (Found.empty()) {
        return PreservedAnalyses::all();
    }

    LLVMContext &Ctx = Md.getContext();
    PointerType *PtrTy = PointerType::getUnqual(Ctx);
    IntegerType *I32Ty = Type::getInt32Ty(Ctx);
    // { annotated value, annotation string, source file, line } - the same
    // 4-field shape clang emits; Utils::getFuncAnnotations only ever reads
    // the first two fields.
    StructType *EntryTy = StructType::get(PtrTy, PtrTy, PtrTy, I32Ty);

    Constant *FileNameConstant =
        createStringConstant(Md, Md.getSourceFileName(), "aspis.rust_annotation_bridge.file");

    std::vector<Constant *> Entries;

    // Preserve any annotations already present (e.g. a module partly
    // compiled from C/C++ and linked with Rust-generated IR).
    if (GlobalVariable *Existing = Md.getGlobalVariable("llvm.global.annotations")) {
        if (auto *CA = dyn_cast<ConstantArray>(Existing->getInitializer())) {
            for (Value *Op : CA->operands()) {
                Entries.push_back(cast<Constant>(Op));
            }
        }
        Existing->eraseFromParent();
    }

    for (const auto &Entry : Found) {
        GlobalValue *AnnotatedGV = Entry.first;
        StringRef Annotation = Entry.second;

        Constant *AnnotationConstant = createStringConstant(
            Md, Annotation, "aspis.rust_annotation_bridge." + Annotation);

        Entries.push_back(ConstantStruct::get(
            EntryTy, {AnnotatedGV, AnnotationConstant, FileNameConstant,
                      ConstantInt::get(I32Ty, 0)}));
    }

    ArrayType *ArrayTy = ArrayType::get(EntryTy, Entries.size());
    auto *NewGlobal =
        new GlobalVariable(Md, ArrayTy, /*isConstant=*/false, GlobalValue::AppendingLinkage,
                            ConstantArray::get(ArrayTy, Entries), "llvm.global.annotations");
    NewGlobal->setSection("llvm.metadata");

    return PreservedAnalyses::none();
}

llvm::PassPluginLibraryInfo getRustAnnotationBridgePluginInfo() {
    return {LLVM_PLUGIN_API_VERSION, "rust-annotation-bridge", LLVM_VERSION_STRING,
            [](PassBuilder &PB) {
                PB.registerPipelineParsingCallback(
                    [](StringRef Name, ModulePassManager &FPM,
                       ArrayRef<PassBuilder::PipelineElement>) {
                        if (Name == "rust-annotation-bridge") {
                            FPM.addPass(RustAnnotationBridge());
                            return true;
                        }
                        return false;
                    });
            }};
}

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
    return getRustAnnotationBridgePluginInfo();
}
