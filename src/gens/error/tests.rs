use super::*;
use crate::gens::IrGenerator;

#[test]
fn errors_are_collected_and_successful_tasks_continue() {
    let mut handle = GenerationErrorHandle::default();
    assert_eq!(
        handle.collect("bad", || fail("missing binding")),
        None::<()>
    );
    assert_eq!(handle.collect("good", || 42), Some(42));
    assert_eq!(
        handle.collect("future", || unsupported("split ABI")),
        None::<()>
    );
    let errors = handle.finish("discard partial output").unwrap_err();
    assert_eq!(errors.errors.len(), 2);
    assert_eq!(errors.errors[0].kind, GenerationErrorKind::Internal);
    assert_eq!(errors.errors[1].kind, GenerationErrorKind::Unsupported);
    assert_eq!(errors.errors[0].task, "bad");
    assert!(errors.to_string().contains("missing binding"));
}

#[test]
fn successful_handle_returns_output() {
    let mut handle = GenerationErrorHandle::default();
    assert_eq!(handle.collect("good", || 7), Some(7));
    assert_eq!(handle.finish(7), Ok(7));
}

#[test]
fn worker_errors_can_be_merged_without_stopping_other_workers() {
    let workers: Vec<_> = (0..3)
        .map(|id| {
            std::thread::spawn(move || {
                let mut handle = GenerationErrorHandle::default();
                handle.collect(format!("file{id}"), || fail("worker failure"));
                assert_eq!(handle.collect("next function", || true), Some(true));
                handle
            })
        })
        .collect();
    let mut merged = GenerationErrorHandle::default();
    for worker in workers {
        merged.append(worker.join().unwrap());
    }
    let errors = merged.finish(()).unwrap_err();
    assert_eq!(
        errors
            .errors
            .iter()
            .map(|e| e.task.as_str())
            .collect::<Vec<_>>(),
        vec!["file0", "file1", "file2"]
    );
}

#[test]
fn generator_rolls_back_buffers_and_names_on_failure() {
    let mut generator = IrGenerator::new("test-triple".into());
    let header = generator.header.clone();
    let result = generator.collect("bad function", |generator| {
        generator.header.push_str("partial header");
        generator.defines.push_str("partial declaration");
        generator.bodys.push_str("partial body");
        generator.remember_symbol("partial name".into());
        fail("generation failed")
    });
    assert_eq!(result, None::<()>);
    assert_eq!(generator.header, header);
    assert!(generator.defines.is_empty());
    assert!(generator.bodys.is_empty());
    assert!(generator.used_symbols.is_empty());
    assert_eq!(
        generator.collect("good function", |generator| {
            generator.bodys.push_str("good body");
            12
        }),
        Some(12)
    );
    assert_eq!(generator.bodys, "good body");
    assert!(generator.finish().is_err());
}

#[test]
fn unexpected_panics_are_recorded_at_the_task_boundary() {
    let mut handle = GenerationErrorHandle::default();
    assert_eq!(
        handle.collect("unexpected", || panic!("broken invariant")),
        None::<()>
    );
    assert_eq!(
        handle.errors()[0].kind,
        GenerationErrorKind::UnexpectedPanic
    );
    assert_eq!(handle.errors()[0].reason, "broken invariant");
    assert_eq!(handle.collect("next", || 1), Some(1));
}
