cmake_minimum_required(VERSION 3.14)
project(PolyGenCppTests VERSION 1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Output directory for generated code
set(GENERATED_DIR "${CMAKE_SOURCE_DIR}/generated")

# Test case directories
set(TEST_CASES
    01_basic_types
    02_imports
    03_nested_namespaces
    04_inline_enums
    05_embedded_structs
    06_arrays_and_optionals
    07_indexes
    08_complex_schema
    09_sqlite
    10_pack_embed
)

# Create test executable for each test case
foreach(TEST_CASE ${TEST_CASES})
    set(TEST_NAME "test_${TEST_CASE}")
    set(TEST_GEN_DIR "${GENERATED_DIR}/${TEST_CASE}/cpp")
    set(TEST_SOURCE "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_${TEST_CASE}.cpp")

    if(NOT EXISTS "${TEST_SOURCE}")
        message(FATAL_ERROR "Test source not found for ${TEST_CASE}: ${TEST_SOURCE}")
    endif()

    if(NOT EXISTS "${TEST_GEN_DIR}")
        message(FATAL_ERROR "Generated code not found for ${TEST_CASE}: ${TEST_GEN_DIR}")
    endif()

    file(GLOB GENERATED_HEADERS "${TEST_GEN_DIR}/*.hpp")
    if(NOT GENERATED_HEADERS)
        message(FATAL_ERROR "Generated C++ headers not found for ${TEST_CASE}: ${TEST_GEN_DIR}")
    endif()

    add_executable(${TEST_NAME}
        ${TEST_SOURCE}
    )

    target_include_directories(${TEST_NAME} PRIVATE
        ${TEST_GEN_DIR}
    )

    # Enable warnings
    target_compile_options(${TEST_NAME} PRIVATE
        $<$<CXX_COMPILER_ID:GNU,Clang>:-Wall -Wextra -pedantic>
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
    )

    message(STATUS "Added test: ${TEST_NAME}")
endforeach()

# Add a target to run all tests
add_custom_target(run_all_tests
    COMMENT "Running all PolyGen C++ tests..."
)

foreach(TEST_CASE ${TEST_CASES})
    set(TEST_NAME "test_${TEST_CASE}")
    if(TARGET ${TEST_NAME})
        add_custom_command(TARGET run_all_tests POST_BUILD
            COMMAND ${TEST_NAME}
            COMMENT "Running ${TEST_NAME}..."
        )
        add_dependencies(run_all_tests ${TEST_NAME})
    endif()
endforeach()
