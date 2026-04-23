// AppError - Contextual error with chain support
export class AppError {
	readonly rootCause: string;
	readonly contextChain: string[];

	constructor(rootCause: string, contextChain: string[] = []) {
		this.rootCause = rootCause;
		this.contextChain = contextChain;
	}

	/**
	 * Add context to this error, returning a new AppError
	 */
	withContext(context: string): AppError {
		return new AppError(this.rootCause, [context, ...this.contextChain]);
	}

	/**
	 * Get the full error message including all context
	 */
	get fullMessage(): string {
		if (this.contextChain.length === 0) {
			return this.rootCause;
		}
		return `${this.contextChain.join(' → ')} → ${this.rootCause}`;
	}

	/**
	 * Get a short display message (first context or root cause)
	 */
	get displayMessage(): string {
		return this.contextChain[0] !== undefined ? this.contextChain[0] : this.rootCause;
	}

	/**
	 * Check if this error has additional context beyond the root cause
	 */
	get hasContext(): boolean {
		return this.contextChain.length > 0;
	}

	toString(): string {
		return this.fullMessage;
	}

	/**
	 * Create an AppError from any error-like value
	 */
	static from(error: unknown): AppError {
		if (error instanceof AppError) {
			return error;
		}
		if (error instanceof Error) {
			return new AppError(error.message);
		}
		if (typeof error === 'string') {
			return new AppError(error);
		}
		return new AppError(String(error));
	}
}

// Result<T, E> - Either a success value or an error
export type Result<T, E = AppError> = OkResult<T, E> | ErrResult<T, E>;
export class OkResult<T, E = AppError> {
	readonly _tag = 'Ok' as const;
	readonly value: T;

	constructor(value: T) {
		this.value = value;
	}

	isOk(): this is OkResult<T, E> {
		return true;
	}

	isErr(): this is ErrResult<T, E> {
		return false;
	}

	/**
	 * Add context to this result (no-op for Ok)
	 */
	context(_context: string): Result<T, AppError> {
		return this as unknown as Result<T, AppError>;
	}

	/**
	 * Map the success value
	 */
	map<U>(fn: (value: T) => U): Result<U, E> {
		return new OkResult(fn(this.value));
	}

	/**
	 * Map the error (no-op for Ok)
	 */
	mapErr<F>(_fn: (error: E) => F): Result<T, F> {
		return this as unknown as Result<T, F>;
	}

	/**
	 * Chain another Result-returning operation
	 */
	andThen<U>(fn: (value: T) => Result<U, E>): Result<U, E> {
		return fn(this.value);
	}

	/**
	 * Get the value or return a default
	 */
	unwrapOr(_defaultValue: T): T {
		return this.value;
	}

	/**
	 * Get the value or compute a default from the error
	 */
	unwrapOrElse(_fn: (error: E) => T): T {
		return this.value;
	}

	/**
	 * Match on the result
	 */
	match<U>(handlers: { ok: (value: T) => U; err: (error: E) => U }): U {
		return handlers.ok(this.value);
	}
}
export class ErrResult<T, E = AppError> {
	readonly _tag = 'Err' as const;
	readonly error: E;

	constructor(error: E) {
		this.error = error;
	}

	isOk(): this is OkResult<T, E> {
		return false;
	}

	isErr(): this is ErrResult<T, E> {
		return true;
	}

	/**
	 * Add context to this error
	 */
	context(context: string): Result<T, AppError> {
		const appError = this.error instanceof AppError ? this.error : AppError.from(this.error);
		return new ErrResult(appError.withContext(context));
	}

	/**
	 * Map the success value (no-op for Err)
	 */
	map<U>(_fn: (value: T) => U): Result<U, E> {
		return this as unknown as Result<U, E>;
	}

	/**
	 * Map the error
	 */
	mapErr<F>(fn: (error: E) => F): Result<T, F> {
		return new ErrResult(fn(this.error));
	}

	/**
	 * Chain another Result-returning operation (no-op for Err)
	 */
	andThen<U>(_fn: (value: T) => Result<U, E>): Result<U, E> {
		return this as unknown as Result<U, E>;
	}

	/**
	 * Get the value or return a default
	 */
	unwrapOr(defaultValue: T): T {
		return defaultValue;
	}

	/**
	 * Get the value or compute a default from the error
	 */
	unwrapOrElse(fn: (error: E) => T): T {
		return fn(this.error);
	}

	/**
	 * Match on the result
	 */
	match<U>(handlers: { ok: (value: T) => U; err: (error: E) => U }): U {
		return handlers.err(this.error);
	}
}

/**
 * Create a successful Result
 */
export function Ok<T>(value: T): OkResult<T, never> {
	return new OkResult(value);
}
/**
 * Create a failed Result
 */
export function Err<E = AppError>(error: E): ErrResult<never, E> {
	return new ErrResult(error);
}
/**
 * Collect an array of Results into a Result of an array
 * Returns Err with the first error if any Result is Err
 */
export function collectResults<T, E>(results: Result<T, E>[]): Result<T[], E> {
	const values: T[] = [];
	for (const result of results) {
		if (result.isErr()) {
			return result as unknown as Result<T[], E>;
		}
		values.push(result.value);
	}
	return Ok(values);
}
/**
 * Wrap a promise that might throw into a Result
 */
export async function tryAsync<T>(
	fn: () => Promise<T>,
	context?: string
): Promise<Result<T, AppError>> {
	try {
		const value = await fn();
		return Ok(value);
	} catch (error) {
		const appError = AppError.from(error);
		return context ? Err(appError.withContext(context)) : Err(appError);
	}
}
/**
 * Wrap a synchronous function that might throw into a Result
 */
export function trySync<T>(fn: () => T, context?: string): Result<T, AppError> {
	try {
		const value = fn();
		return Ok(value);
	} catch (error) {
		const appError = AppError.from(error);
		return context ? Err(appError.withContext(context)) : Err(appError);
	}
}

// Option<T> - Either a value or nothing
export type Option<T> = SomeOption<T> | NoneOption<T>;
export class SomeOption<T> {
	readonly _tag = 'Some' as const;
	readonly value: T;

	constructor(value: T) {
		this.value = value;
	}

	isSome(): this is SomeOption<T> {
		return true;
	}

	isNone(): this is NoneOption<T> {
		return false;
	}

	/**
	 * Map the value if Some
	 */
	map<U>(fn: (value: T) => U): Option<U> {
		return new SomeOption(fn(this.value));
	}

	/**
	 * Chain another Option-returning operation
	 */
	andThen<U>(fn: (value: T) => Option<U>): Option<U> {
		return fn(this.value);
	}

	/**
	 * Get the value or return a default
	 */
	unwrapOr(_defaultValue: T): T {
		return this.value;
	}

	/**
	 * Get the value or compute a default
	 */
	unwrapOrElse(_fn: () => T): T {
		return this.value;
	}

	/**
	 * Convert to Result with error if None
	 */
	okOr<E>(_error: E): Result<T, E> {
		return new OkResult(this.value);
	}

	/**
	 * Convert to Result with error from function if None
	 */
	okOrElse<E>(_fn: () => E): Result<T, E> {
		return new OkResult(this.value);
	}

	/**
	 * Match on the option
	 */
	match<U>(handlers: { some: (value: T) => U; none: () => U }): U {
		return handlers.some(this.value);
	}

	/**
	 * Filter the value, returning None if predicate is false
	 */
	filter(predicate: (value: T) => boolean): Option<T> {
		return predicate(this.value) ? this : None();
	}

	/**
	 * Convert to nullable value (T | null)
	 */
	toNullable(): T | null {
		return this.value;
	}
}
export class NoneOption<T> {
	readonly _tag = 'None' as const;

	isSome(): this is SomeOption<T> {
		return false;
	}

	isNone(): this is NoneOption<T> {
		return true;
	}

	/**
	 * Map the value if Some (no-op for None)
	 */
	map<U>(_fn: (value: T) => U): Option<U> {
		return this as unknown as NoneOption<U>;
	}

	/**
	 * Chain another Option-returning operation (no-op for None)
	 */
	andThen<U>(_fn: (value: T) => Option<U>): Option<U> {
		return this as unknown as NoneOption<U>;
	}

	/**
	 * Get the value or return a default
	 */
	unwrapOr(defaultValue: T): T {
		return defaultValue;
	}

	/**
	 * Get the value or compute a default
	 */
	unwrapOrElse(fn: () => T): T {
		return fn();
	}

	/**
	 * Convert to Result with error if None
	 */
	okOr<E>(error: E): Result<T, E> {
		return new ErrResult(error);
	}

	/**
	 * Convert to Result with error from function if None
	 */
	okOrElse<E>(fn: () => E): Result<T, E> {
		return new ErrResult(fn());
	}

	/**
	 * Match on the option
	 */
	match<U>(handlers: { some: (value: T) => U; none: () => U }): U {
		return handlers.none();
	}

	/**
	 * Filter the value (always returns None for None)
	 */
	filter(_predicate: (value: T) => boolean): Option<T> {
		return this;
	}

	/**
	 * Convert to nullable value (T | null)
	 */
	toNullable(): T | null {
		return null;
	}
}

/**
 * Create a Some option with a value
 */
export function Some<T>(value: T): SomeOption<T> {
	return new SomeOption(value);
}
/**
 * Create a None option
 */
export function None<T = never>(): NoneOption<T> {
	return new NoneOption();
}
/**
 * Create an Option from a nullable value
 */
export function fromNullable<T>(value: T | null | undefined): Option<T> {
	return value === null || value === undefined ? None() : Some(value);
}
/**
 * Collect an array of Options into an Option of an array
 * Returns None if any Option is None
 */
export function collectOptions<T>(options: Option<T>[]): Option<T[]> {
	const values: T[] = [];
	for (const option of options) {
		if (option.isNone()) {
			return None();
		}
		values.push(option.value);
	}
	return Some(values);
}
