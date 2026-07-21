fn mix(acc: i64, n: i64) -> i64 {
    let m = acc * 31 + n;
    m - (m / 1000003 * 1000003)
}

fn burn(n: i64, acc: i64) -> i64 {
    if n == 0 { acc } else { burn(n - 1, mix(acc, n)) }
}

fn main() {
    let mut total: i64 = 0;
    for i in (1..=100000i64).rev() {
        // the hand-restructured form: only compute what the branch demands
        if i % 20 == 1 {
            total += burn(5000, std::hint::black_box(i));
        }
    }
    println!("sum {total}");
}
