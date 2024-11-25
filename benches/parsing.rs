use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wpilog_parser::read_only::WPILog;

fn benchmark(c: &mut Criterion) {
    let data = include_bytes!("../examples/parsing/Log_24-04-06_13-28-45_e5.wpilog");
    c.bench_function("parse", |b| b.iter(|| WPILog::parse(black_box(data))));
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
