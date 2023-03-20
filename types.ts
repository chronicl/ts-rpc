type A1 = [number];
type A2 = [number, number];

type Result<T, E> = { Ok: T } | { Err: E };
