#pragma once

typedef unsigned char uint8_t;
typedef unsigned short uint16_t;
typedef unsigned int uint;
typedef unsigned long long int uint64_t;

typedef signed char int8_t;
typedef signed short int16_t;
typedef signed int int32_t;
typedef signed long long int64_t;

struct float2 {
  float x, y;
};

struct float3 {
  float x, y, z;
};

struct float4 {
  float x, y, z, w;
};

struct float4x4 {
  float4 col0;
  float4 col1;
  float4 col2;
  float4 col3;
};
