use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nexus_common::{Crypto, NodeIdentity, SealedEnvelope};

fn bench_aes256gcm_encrypt_decrypt(c: &mut Criterion) {
    let crypto = Crypto::new(Crypto::generate_key());
    let payload = "a]".repeat(512);

    c.bench_function("aes256gcm_encrypt_1kb", |b| {
        b.iter(|| crypto.encrypt(black_box(&payload)).unwrap())
    });

    let encrypted = crypto.encrypt(&payload).unwrap();
    c.bench_function("aes256gcm_decrypt_1kb", |b| {
        b.iter(|| crypto.decrypt(black_box(&encrypted)).unwrap())
    });
}

fn bench_sealed_envelope(c: &mut Criterion) {
    let sender = NodeIdentity::generate();
    let recipient = NodeIdentity::generate();
    let recipient_x25519 = recipient.x25519_public();
    let payload = vec![0xABu8; 1024];

    c.bench_function("sealed_envelope_seal_1kb", |b| {
        b.iter(|| {
            SealedEnvelope::seal(
                black_box(&sender),
                black_box(&recipient_x25519),
                black_box(&payload),
            )
            .unwrap()
        })
    });

    let envelope = SealedEnvelope::seal(&sender, &recipient_x25519, &payload).unwrap();
    c.bench_function("sealed_envelope_open_1kb", |b| {
        b.iter(|| black_box(&envelope).open(black_box(&recipient)).unwrap())
    });
}

criterion_group!(benches, bench_aes256gcm_encrypt_decrypt, bench_sealed_envelope);
criterion_main!(benches);
