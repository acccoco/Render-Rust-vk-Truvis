cmake_minimum_required(VERSION 3.10)

project(TruvisAssimp)

# 强制静态链接设置
# set(VCPKG_TARGET_TRIPLET "x64-windows-static" CACHE STRING "")
# set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")

# msbuild 乱码问题
add_compile_options("$<$<C_COMPILER_ID:MSVC>:/utf-8>")
add_compile_options("$<$<CXX_COMPILER_ID:MSVC>:/utf-8>")

# 从当前目录开始查找 assimp
find_package(assimp REQUIRED CONFIG)

add_library(truvis_assimp SHARED src/lib.cpp)
target_include_directories(truvis_assimp PUBLIC include)
target_link_libraries(truvis_assimp PUBLIC assimp::assimp)
target_compile_definitions(truvis_assimp PRIVATE BUILDING_DLL)

add_executable(main src/main.cpp)
target_include_directories(main PUBLIC include)
target_link_libraries(main PUBLIC truvis_assimp)

# 安装头文件
# 其实可以啥也不用安装，只是确保存在 install 这个 target 让 rust-cmake 调用，相当于 BUILD_ALL
install(DIRECTORY include/ DESTINATION include)