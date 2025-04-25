#pragma once

#ifdef BUILDING_DLL
#define DLL_API __declspec(dllexport)
#else
#define DLL_API
#endif

// dllimport 不是必须的，因为有 导入库 .lib 告诉连接器哪些符号需要动态链接

extern "C" {
DLL_API unsigned int get_vert_cnts();
}
