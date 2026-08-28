cmake_minimum_required(VERSION 3.22)

project(ASPIS DESCRIPTION "ASPIS - Automatic Software-based Protection to Improve Safety")

find_package(LLVM REQUIRED CONFIG)

list(APPEND CMAKE_MODULE_PATH "${LLVM_CMAKE_DIR}")
include(AddLLVM)

message(STATUS "Using LLVMConfig.cmake in: ${LLVM_DIR}")
message(STATUS "Found LLVM ${LLVM_PACKAGE_VERSION}")
message(STATUS "Using LLVM Cmake: ${LLVM_CMAKE_DIR}")


separate_arguments(LLVM_DEFINITIONS_LIST NATIVE_COMMAND ${LLVM_DEFINITIONS})
add_definitions(${LLVM_DEFINITIONS_LIST})
include_directories(SYSTEM ${LLVM_INCLUDE_DIRS})

# LLVM is normally built without RTTI. Be consistent with that.
#if(NOT LLVM_ENABLE_RTTI)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti")
#endif()

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++20")

message(STATUS "Found LLVM definitions ${LLVM_DEFINITIONS_LIST}")
message(STATUS "Found LLVM include dirs ${LLVM_INCLUDE_DIRS}")
message(STATUS "RTTI: ${LLVM_ENABLE_RTTI}")

add_subdirectory(libs/TypeDeductionAnalysis)
add_subdirectory(passes)

# Rust front-end support: builds the aspis-annotations rlib that aspis.sh
# links Rust test inputs against (see rust-annotations/src/lib.rs). Requires
# the rust-toolchain.toml-pinned rustc/cargo on PATH.
find_program(CARGO_EXECUTABLE cargo)
if(CARGO_EXECUTABLE)
  add_custom_target(rust_annotations ALL
    COMMAND ${CARGO_EXECUTABLE} build --release --manifest-path ${CMAKE_SOURCE_DIR}/rust-annotations/Cargo.toml
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}/rust-annotations
    COMMENT "Building aspis-annotations rlib"
    VERBATIM
  )
else()
  message(STATUS "cargo not found - Rust test inputs (aspis.sh *.rs) will not be available")
endif()