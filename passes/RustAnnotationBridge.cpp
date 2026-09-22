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
#include <iostream>

using namespace llvm;

#define DEBUG_TYPE "aspis-rust-annotation-bridge"

class RustAnnotationBridge : public PassInfoMixin<RustAnnotationBridge>
{
public:
    PreservedAnalyses run(Module &Md, ModuleAnalysisManager &)
    {
        std::cout << "Running inside RustAnnotationBridge pass" << std::endl;

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
