
# 第三方库
###########################################################################
find_package(assimp CONFIG REQUIRED)
find_package(glm CONFIG REQUIRED)


# target
###########################################################################
file(GLOB_RECURSE SOURCE_FILES ${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp)
file(GLOB_RECURSE HEADER_FILES ${CMAKE_CURRENT_SOURCE_DIR}/include/TruvixxAssimp/*.hpp)
add_library(truvixx-assimp SHARED
        ${SOURCE_FILES}
        ${HEADER_FILES}
)

# 定义导入/导出宏
include(GenerateExportHeader)
generate_export_header(truvixx-assimp
        BASE_NAME TRUVIXX_ASSIMP
        EXPORT_MACRO_NAME TRUVIXX_ASSIMP_API
        EXPORT_FILE_NAME ${CMAKE_CURRENT_SOURCE_DIR}/include/TruvixxAssimp/truvixx_assimp.export.h
        STATIC_DEFINE TRUVIXX_ASSIMP_STATIC_DEFINE
)


target_include_directories(truvixx-assimp PUBLIC include)
target_link_libraries(truvixx-assimp PUBLIC
        assimp::assimp
        glm::glm
)

