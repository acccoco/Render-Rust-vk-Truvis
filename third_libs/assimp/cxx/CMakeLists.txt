cmake_minimum_required(VERSION 3.10)

project(TruvisAssimp)


find_package(assimp CONFIG REQUIRED)


add_library(ass STATIC src/lib.cpp)
target_include_directories(ass PUBLIC include)
target_link_libraries(ass PRIVATE assimp::assimp)
