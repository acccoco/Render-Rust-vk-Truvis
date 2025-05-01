cmake_minimum_required(VERSION 3.10)

# 命令行需要指定的参数：
# -Thost=x64 -Ax64

project(TruvisAssimp)

# 从当前目录开始查找 assimp
find_package(assimp REQUIRED CONFIG)

add_library(truvis-assimp SHARED src/lib.cpp)
target_include_directories(truvis-assimp PUBLIC include)
target_link_libraries(truvis-assimp PUBLIC assimp::assimp)
target_compile_definitions(truvis-assimp PRIVATE BUILDING_DLL)

add_executable(main src/main.cpp)
target_include_directories(main PUBLIC include)
target_link_libraries(main PUBLIC truvis-assimp)

# 安装
# 使用 CODE 可以确保读取到的文件是运行时的
install(CODE [[
    file(GLOB DEBUG_DLLS ${CMAKE_BINARY_DIR}/Debug/*.dll)
    file(GLOB DEBUG_LIBS ${CMAKE_BINARY_DIR}/Debug/*.lib)
    message(STATUS "install dlls(debug): ${DEBUG_DLLS}")
    message(STATUS "install libs(debug): ${DEBUG_LIBS}")

    file(INSTALL ${DEBUG_DLLS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Debug/bin")
    file(INSTALL ${DEBUG_LIBS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Debug/lib")

    file(GLOB RELEASE_DLLS ${CMAKE_BINARY_DIR}/Release/*.dll)
    file(GLOB RELEASE_LIBS ${CMAKE_BINARY_DIR}/Release/*.lib)
    message(STATUS "install dlls(release): ${RELEASE_DLLS}")
    message(STATUS "install libs(release): ${RELEASE_LIBS}")

    file(INSTALL ${RELEASE_DLLS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Release/bin")
    file(INSTALL ${RELEASE_LIBS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Release/lib")
]])
