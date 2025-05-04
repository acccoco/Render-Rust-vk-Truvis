cmake_minimum_required(VERSION 3.10)

set(CMAKE_TOOLCHAIN_FILE "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake" CACHE STRING "")
set(CMAKE_EXPORT_COMPILE_COMMANDS ON CACHE INTERNAL "")

project(TruvisAssimpCxx)


# compile options
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall")


# 第三方库
###########################################################################
find_package(assimp CONFIG REQUIRED)
find_package(glm CONFIG REQUIRED)


# 动态库
###########################################################################
file(GLOB_RECURSE SRC_FILES ${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp)
add_library(truvis-assimp-cxx SHARED ${SRC_FILES})
target_include_directories(truvis-assimp-cxx PUBLIC include)
target_link_libraries(truvis-assimp-cxx PUBLIC
        assimp::assimp
        glm::glm
)
# 定义特殊的宏，表示某些符号需要导出到 dll
target_compile_definitions(truvis-assimp-cxx PRIVATE BUILDING_DLL)


# target main
###########################################################################
add_executable(main main.cpp)
target_link_libraries(main truvis-assimp-cxx)


# 安装
###########################################################################
# 使用 CODE 可以确保读取到的文件是运行时的
install(CODE [[
    # debug
    file(GLOB DEBUG_DLLS ${CMAKE_BINARY_DIR}/Debug/*.dll)
    file(GLOB DEBUG_LIBS ${CMAKE_BINARY_DIR}/Debug/*.lib)
    file(GLOB DEBUG_PDBS ${CMAKE_BINARY_DIR}/Debug/*.pdb)
    message(STATUS "install dlls(debug): ${DEBUG_DLLS}")
    message(STATUS "install libs(debug): ${DEBUG_LIBS}")
    message(STATUS "install pdbs(debug): ${DEBUG_PDBS}")

    file(INSTALL ${DEBUG_DLLS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Debug/bin")
    file(INSTALL ${DEBUG_LIBS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Debug/lib")
    file(INSTALL ${DEBUG_PDBS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Debug/bin")

    # release
    file(GLOB RELEASE_DLLS ${CMAKE_BINARY_DIR}/Release/*.dll)
    file(GLOB RELEASE_LIBS ${CMAKE_BINARY_DIR}/Release/*.lib)
    file(GLOB RELEASE_PDBS ${CMAKE_BINARY_DIR}/Release/*.pdb)
    message(STATUS "install dlls(release): ${RELEASE_DLLS}")
    message(STATUS "install libs(release): ${RELEASE_LIBS}")
    message(STATUS "install pdbs(release): ${RELEASE_PDBS}")

    file(INSTALL ${RELEASE_DLLS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Release/bin")
    file(INSTALL ${RELEASE_LIBS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Release/lib")
    file(INSTALL ${RELEASE_PDBS} DESTINATION "${CMAKE_INSTALL_PREFIX}/Release/bin")
]])
