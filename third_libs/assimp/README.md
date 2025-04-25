目录结构

* $OUTDIR = $PROJECT/target/debug/build/$CRATE-$HASH/out
* 其中：build/Debug 或者 build/Release 就是存放 lib, dll, exe, pdb 的位置

```
// 没有 type 就表示是 dll
// 甚至都不需要 这个 link 属性，因为 dll 的导入库 .lib 已经在 build.rs 中指定需要链接了
// ，所以 linker 知道这个符号需要从 dll 中加载
// #[link(name = "truvis-assimp")]
// extern "C" {
//     fn get_vert_cnts() -> u32;
// }
```
