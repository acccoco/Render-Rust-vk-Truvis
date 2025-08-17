#pragma once

#ifdef BUILDING_DLL    // 该 macro 由 cmake 定义
#define DLL_API __declspec(dllexport)
#else
// dllimport 不是必须的，因为有 导入库 .lib 告诉连接器哪些符号需要动态链接
#define DLL_API __declspec(dllimport)
#endif
