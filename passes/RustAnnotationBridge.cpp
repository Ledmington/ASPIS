/**
 * ************************************************************************************************
 * @brief  LLVM pass that translates the markers emitted by the `#[aspis(...)]` Rust proc-macro
 *         (see rust-annotations/src/lib.rs) into regular `llvm.global.annotations` entries, the
 *         same form clang emits for `__attribute__((annotate(...)))`.
 *
 *         Every marker is a global in the "aspis.annotations" section whose initializer is a
 *         `{ ptr @annotated_symbol, ptr @"annotation\0" }` struct. Markers are removed from
 *         `llvm.used` and deleted once translated, so the module no longer references them.
 * ************************************************************************************************
 */
#include "llvm/IR/Constants.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/GlobalVariable.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Support/Debug.h"
#include "llvm/Transforms/Utils/ModuleUtils.h"

#include <vector>

using namespace llvm;

#define DEBUG_TYPE "aspis-rust-annotation-bridge"

static constexpr StringRef MarkerSection = "aspis.annotations";

class RustAnnotationBridge : public PassInfoMixin<RustAnnotationBridge>
{
public:
    PreservedAnalyses run(Module &Md, ModuleAnalysisManager &)
    {
        std::vector<GlobalVariable *> Markers;
        for (GlobalVariable &GV : Md.globals())
        {
            if (GV.hasSection() && GV.getSection() == MarkerSection)
            {
                Markers.push_back(&GV);
            }
        }

        if (Markers.empty())
        {
            return PreservedAnalyses::all();
        }

        LLVMContext &Ctx = Md.getContext();
        PointerType *PtrTy = PointerType::getUnqual(Ctx);

        // Keep any annotation already in the module (e.g. from linked C/C++ sources) and reuse its
        // element type; otherwise use clang's layout: { annotated, string, file, line, args }.
        std::vector<Constant *> Entries;
        StructType *EntryTy = nullptr;
        if (GlobalVariable *GA = Md.getGlobalVariable("llvm.global.annotations"))
        {
            if (auto *CA = dyn_cast<ConstantArray>(GA->getInitializer()))
            {
                for (Use &Op : CA->operands())
                {
                    Entries.push_back(cast<Constant>(Op.get()));
                }
            }
            EntryTy = dyn_cast<StructType>(GA->getValueType()->getArrayElementType());
            GA->eraseFromParent();
        }
        if (!EntryTy)
        {
            EntryTy = StructType::get(Ctx, {PtrTy, PtrTy, PtrTy, Type::getInt32Ty(Ctx), PtrTy});
        }

        for (GlobalVariable *Marker : Markers)
        {
            auto *Init = dyn_cast_or_null<ConstantStruct>(
                Marker->hasInitializer() ? Marker->getInitializer() : nullptr);
            if (!Init || Init->getNumOperands() != 2)
            {
                report_fatal_error("Malformed ASPIS marker '" + Marker->getName() + "'");
            }

            auto *Target = dyn_cast<GlobalValue>(Init->getOperand(0)->stripPointerCasts());
            auto *String = dyn_cast<GlobalVariable>(Init->getOperand(1)->stripPointerCasts());
            if (!Target || !String || !String->hasInitializer() ||
                !isa<ConstantDataArray>(String->getInitializer()))
            {
                report_fatal_error("Malformed ASPIS marker '" + Marker->getName() + "'");
            }

            LLVM_DEBUG(dbgs() << "Annotating " << Target->getName() << " with "
                              << cast<ConstantDataArray>(String->getInitializer())->getAsCString()
                              << "\n");

            std::vector<Constant *> Fields = {Target, String};
            for (unsigned I = 2; I < EntryTy->getNumElements(); I++)
            {
                Fields.push_back(Constant::getNullValue(EntryTy->getElementType(I)));
            }
            Entries.push_back(ConstantStruct::get(EntryTy, Fields));
        }

        SmallPtrSet<Constant *, 8> MarkerSet(Markers.begin(), Markers.end());
        removeFromUsedLists(Md, [&](Constant *C)
                            { return MarkerSet.contains(C); });
        for (GlobalVariable *Marker : Markers)
        {
            Marker->eraseFromParent();
        }

        ArrayType *ArrTy = ArrayType::get(EntryTy, Entries.size());
        auto *GA = new GlobalVariable(Md, ArrTy, false, GlobalValue::AppendingLinkage,
                                      ConstantArray::get(ArrTy, Entries), "llvm.global.annotations");
        GA->setSection("llvm.metadata");

        return PreservedAnalyses::none();
    }

    static bool isRequired() { return true; }
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
