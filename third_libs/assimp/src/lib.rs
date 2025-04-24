// 没有 type 就表示是 dll
#[link(name = "truvis-assimp")]
extern "C" {
    fn get_vert_cnts() -> u32;
}

pub fn foo() -> u32 {
    unsafe { get_vert_cnts() }
}
