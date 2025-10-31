use task_queue_rs::TaskQueueError;

#[test]
fn test_error_types() {
    let err = TaskQueueError::QueueEmpty;
    assert_eq!(err.to_string(), "Queue is empty");

    let err = TaskQueueError::TaskNotFound("test-id".to_string());
    assert_eq!(err.to_string(), "Task not found: test-id");
}
