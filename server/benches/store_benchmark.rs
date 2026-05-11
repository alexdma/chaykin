use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use chaykin::store::{NaiveStore, IndexedStore, TripleStore};

/// Generate synthetic Turtle data with the given number of subjects,
/// predicates per subject, and objects per predicate.
fn generate_turtle(num_subjects: usize, preds_per_subject: usize, objs_per_pred: usize) -> String {
    let mut turtle = String::new();
    for s in 0..num_subjects {
        for p in 0..preds_per_subject {
            for o in 0..objs_per_pred {
                turtle.push_str(&format!(
                    "<http://example.org/s{}> <http://example.org/p{}> \"object_{}_{}_{}\" .\n",
                    s, p, s, p, o
                ));
            }
        }
    }
    turtle
}

/// Total triple count for a given configuration.
fn triple_count(num_subjects: usize, preds_per_subject: usize, objs_per_pred: usize) -> usize {
    num_subjects * preds_per_subject * objs_per_pred
}

fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("load");

    let configs: Vec<(usize, usize, usize)> = vec![
        (10, 5, 2),    // 100 triples
        (100, 5, 2),   // 1,000 triples
        (1000, 5, 2),  // 10,000 triples
    ];

    for &(ns, np, no) in &configs {
        let total = triple_count(ns, np, no);
        let turtle = generate_turtle(ns, np, no);

        group.bench_with_input(
            BenchmarkId::new("NaiveStore", total),
            &turtle,
            |b, data| {
                b.iter(|| {
                    let mut store = NaiveStore::new();
                    store.load_from_string(data).unwrap();
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("IndexedStore", total),
            &turtle,
            |b, data| {
                b.iter(|| {
                    let mut store = IndexedStore::new();
                    store.load_from_string(data).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("lookup");

    let configs: Vec<(usize, usize, usize)> = vec![
        (10, 5, 2),    // 100 triples
        (100, 5, 2),   // 1,000 triples
        (1000, 5, 2),  // 10,000 triples
    ];

    for &(ns, np, no) in &configs {
        let total = triple_count(ns, np, no);
        let turtle = generate_turtle(ns, np, no);
        // Look up a subject in the middle of the dataset
        let target = format!("http://example.org/s{}", ns / 2);

        let mut naive = NaiveStore::new();
        naive.load_from_string(&turtle).unwrap();

        let mut indexed = IndexedStore::new();
        indexed.load_from_string(&turtle).unwrap();

        group.bench_with_input(
            BenchmarkId::new("NaiveStore", total),
            &target,
            |b, iri| {
                b.iter(|| naive.get_resource_description(iri));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("IndexedStore", total),
            &target,
            |b, iri| {
                b.iter(|| indexed.get_resource_description(iri));
            },
        );
    }

    group.finish();
}

fn bench_all_subjects(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_subjects");

    let configs: Vec<(usize, usize, usize)> = vec![
        (10, 5, 2),
        (100, 5, 2),
        (1000, 5, 2),
    ];

    for &(ns, np, no) in &configs {
        let total = triple_count(ns, np, no);
        let turtle = generate_turtle(ns, np, no);

        let mut naive = NaiveStore::new();
        naive.load_from_string(&turtle).unwrap();

        let mut indexed = IndexedStore::new();
        indexed.load_from_string(&turtle).unwrap();

        group.bench_with_input(
            BenchmarkId::new("NaiveStore", total),
            &(),
            |b, _| {
                b.iter(|| naive.get_all_subjects());
            },
        );

        group.bench_with_input(
            BenchmarkId::new("IndexedStore", total),
            &(),
            |b, _| {
                b.iter(|| indexed.get_all_subjects());
            },
        );
    }

    group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");

    let configs: Vec<(usize, usize, usize)> = vec![
        (10, 5, 2),
        (100, 5, 2),
        (1000, 5, 2),
    ];

    for &(ns, np, no) in &configs {
        let total = triple_count(ns, np, no);
        let turtle = generate_turtle(ns, np, no);
        let target = format!("http://example.org/s{}", ns / 2);

        group.bench_with_input(
            BenchmarkId::new("NaiveStore", total),
            &turtle,
            |b, data| {
                b.iter(|| {
                    let mut store = NaiveStore::new();
                    store.load_from_string(data).unwrap();
                    let desc = store.get_resource_description(&target);
                    // Simulate condensed-mode grouping that gemtext.rs does
                    let mut grouped: std::collections::HashMap<String, Vec<chaykin::store::RdfNode>> =
                        std::collections::HashMap::new();
                    for (pred, obj) in &desc {
                        grouped.entry(pred.clone()).or_default().push(obj.clone());
                    }
                    let mut keys: Vec<_> = grouped.keys().cloned().collect();
                    keys.sort();
                    keys
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("IndexedStore", total),
            &turtle,
            |b, data| {
                b.iter(|| {
                    let mut store = IndexedStore::new();
                    store.load_from_string(data).unwrap();
                    let desc = store.get_resource_description(&target);
                    // Same condensed-mode grouping for fair comparison
                    let mut grouped: std::collections::HashMap<String, Vec<chaykin::store::RdfNode>> =
                        std::collections::HashMap::new();
                    for (pred, obj) in &desc {
                        grouped.entry(pred.clone()).or_default().push(obj.clone());
                    }
                    let mut keys: Vec<_> = grouped.keys().cloned().collect();
                    keys.sort();
                    keys
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_load, bench_lookup, bench_all_subjects, bench_end_to_end);
criterion_main!(benches);
