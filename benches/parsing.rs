use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wpilog_parser::parse_record;

fn benchmark(c: &mut Criterion) {
    const DATA: [u8; 0x20] = [
        0x20, // Timestamp Length = 3, Payload Size Length = 1, Entry Id Length = 1
        0x00, // Entry Id = 0
        0x1A, // Payload Size = 26
        0x40, 0x42, 0x0F, // Timestamp = 1_000_000
        0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, // Payload = [u8; 26]
        0x00, 0x00, 0x74, 0x65, 0x73, 0x74, 0x05, //
        0x00, 0x00, 0x00, 0x69, 0x6E, 0x74, 0x36, //
        0x34, 0x00, 0x00, 0x00, 0x00,
    ];

    c.bench_function("single record", |b| {
        b.iter(|| parse_record(black_box(&DATA)))
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
