#include <iostream>

#include "lib.hpp"

int main() {
    auto cnt = get_vert_cnts();

    std::cout << "Total vertices: " << cnt << '\n';
    return 0;
}
