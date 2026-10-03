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

# We do not control the TypeDeductionAnalysis repository, so our local change to it is kept as a
# patch and applied to the submodule's working tree at configure time (skipped if already applied).
find_package(Git REQUIRED)
set(TDA_DIR "${CMAKE_CURRENT_SOURCE_DIR}/libs/TypeDeductionAnalysis")
set(TDA_PATCH "${CMAKE_CURRENT_SOURCE_DIR}/patches/tda-max-iterations.patch")
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS ${TDA_PATCH})
execute_process(
  COMMAND ${GIT_EXECUTABLE} apply --reverse --check ${TDA_PATCH}
  WORKING_DIRECTORY ${TDA_DIR}
  RESULT_VARIABLE TDA_PATCH_ALREADY_APPLIED
  OUTPUT_QUIET ERROR_QUIET
)
if(TDA_PATCH_ALREADY_APPLIED EQUAL 0)
  message(STATUS "TDA patch already applied: ${TDA_PATCH}")
else()
  execute_process(
    COMMAND ${GIT_EXECUTABLE} apply ${TDA_PATCH}
    WORKING_DIRECTORY ${TDA_DIR}
    RESULT_VARIABLE TDA_PATCH_RESULT
  )
  if(NOT TDA_PATCH_RESULT EQUAL 0)
    message(FATAL_ERROR "Could not apply TDA patch: ${TDA_PATCH}")
  endif()
  message(STATUS "Applied TDA patch: ${TDA_PATCH}")
endif()

add_subdirectory(libs/TypeDeductionAnalysis)
add_subdirectory(passes)