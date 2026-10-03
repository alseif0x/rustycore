cmake_minimum_required(VERSION 3.23)
project(forever_client_data_probe LANGUAGES C CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT RUSTYCORE_FOREVER_CPP_REF)
  message(FATAL_ERROR "Provide the pinned Forever reference checkout")
endif()
execute_process(COMMAND git -C "${RUSTYCORE_FOREVER_CPP_REF}" rev-parse HEAD
  OUTPUT_VARIABLE reference_sha OUTPUT_STRIP_TRAILING_WHITESPACE
  RESULT_VARIABLE reference_status)
if(NOT reference_status EQUAL 0 OR
   NOT reference_sha STREQUAL "02245dcd245e7433e524577656177723d3e4992e")
  message(FATAL_ERROR "Unverified Forever reference revision")
endif()
execute_process(COMMAND git -C "${RUSTYCORE_FOREVER_CPP_REF}" ls-tree
  -r --name-only HEAD -- dep/CascLib OUTPUT_VARIABLE reference_files
  OUTPUT_STRIP_TRAILING_WHITESPACE RESULT_VARIABLE reference_listing)
if(NOT reference_listing EQUAL 0 OR NOT reference_files)
  message(FATAL_ERROR "Cannot enumerate pinned CascLib sources")
endif()
string(REPLACE "\n" ";" reference_files "${reference_files}")
# A source-only/sparse reference can have a deliberately empty index. Check
# compiler inputs against HEAD blobs, not that unrelated index's dirty status.
foreach(relative_path IN LISTS reference_files)
  if(IS_SYMLINK "${RUSTYCORE_FOREVER_CPP_REF}/${relative_path}")
    message(FATAL_ERROR "Unverified CascLib source symlink")
  endif()
  execute_process(COMMAND git -C "${RUSTYCORE_FOREVER_CPP_REF}" rev-parse
    "HEAD:${relative_path}" OUTPUT_VARIABLE expected_blob
    OUTPUT_STRIP_TRAILING_WHITESPACE RESULT_VARIABLE expected_status)
  execute_process(COMMAND git -C "${RUSTYCORE_FOREVER_CPP_REF}" hash-object
    "${RUSTYCORE_FOREVER_CPP_REF}/${relative_path}" OUTPUT_VARIABLE actual_blob
    OUTPUT_STRIP_TRAILING_WHITESPACE ERROR_QUIET RESULT_VARIABLE actual_status)
  if(NOT expected_status EQUAL 0 OR NOT actual_status EQUAL 0 OR
     NOT expected_blob STREQUAL actual_blob)
    message(FATAL_ERROR "CascLib compiler input differs: ${relative_path}")
  endif()
endforeach()

find_package(ZLIB REQUIRED)
find_package(Threads REQUIRED)
add_library(trinity-dependency-interface INTERFACE)
add_library(zlib ALIAS ZLIB::ZLIB)
add_subdirectory("${RUSTYCORE_FOREVER_CPP_REF}/dep/CascLib" casc)
add_executable(forever-client-data-probe main.cpp)
target_link_libraries(forever-client-data-probe PRIVATE casc Threads::Threads)
get_filename_component(private_root "${CMAKE_CURRENT_SOURCE_DIR}/../../../target/forever-login" ABSOLUTE)
target_compile_definitions(forever-client-data-probe PRIVATE RUSTYCORE_QA_ROOT="${private_root}")
add_executable(forever-initial-gt-oracle gt_oracle.cpp)
target_compile_definitions(forever-initial-gt-oracle PRIVATE RUSTYCORE_QA_ROOT="${private_root}")
enable_testing()
add_test(NAME wdc5-header-contract COMMAND forever-client-data-probe --self-test)
