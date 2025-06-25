cmake_minimum_required(VERSION 3.31)
project(vtd_c_example)

set(CMAKE_C_STANDARD 23)

function(find_libum_ffi)
    set(RUST_BUILD_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/../../../target/debug/")
    add_library(vtd_libum_ffi SHARED IMPORTED)

    set_target_properties(vtd_libum_ffi PROPERTIES
        IMPORTED_LOCATION "${RUST_BUILD_DIRECTORY}/vtd_libum_ffi.dll"
        IMPORTED_IMPLIB "${RUST_BUILD_DIRECTORY}/vtd_libum_ffi.dll.lib"
        INTERFACE_INCLUDE_DIRECTORIES "${RUST_BUILD_DIRECTORY}/generated/include"
    )
endfunction()

find_libum_ffi()

add_executable(vtd_c_example main.c)
target_link_libraries(vtd_c_example vtd_libum_ffi)

