# 使用 VisualStudio 作为项目 generator

```shell
cmake -G "Visual Studio 17 2022" -A x64 -DCMAKE_CONFIGURATION_TYPES="Debug;Release" ..
```

# 使用 clang-cl 作为项目 generator

- c compiler: clang-cl
- c++ compiler: clang-cl
- generator: "Ninja"

对应的 cmake 命令为：

```shell
cmake `
  -DCMAKE_BUILD_TYPE=Debug `
  -DCMAKE_MAKE_PROGRAM=C:/Users/bigso/AppData/Local/Microsoft/WinGet/Links/ninja.exe `
  "-DCMAKE_C_COMPILER=C:/Program Files/LLVM/bin/clang-cl.exe" `
  "-DCMAKE_CXX_COMPILER=C:/Program Files/LLVM/bin/clang-cl.exe" `
  -G Ninja `
  -S D:\code\Render-Rust-vk-Truvis\crates\truvis-cxx\cxx `
  -B D:\code\Render-Rust-vk-Truvis\crates\truvis-cxx\cxx\build-clang
```
