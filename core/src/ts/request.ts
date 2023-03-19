export type ServerConfig = {
  url: string;
};

export type RequestOptions = {
  readonly method: 'GET' | 'PUT' | 'POST' | 'DELETE' | 'OPTIONS' | 'HEAD' | 'PATCH';
  readonly url: string;
  readonly path?: Record<string, any>;
  readonly cookies?: Record<string, any>;
  readonly headers?: Record<string, any>;
  readonly query?: Record<string, any>;
  readonly formData?: Record<string, any>;
  readonly body?: any;
  readonly mediaType?: string;
  readonly responseHeader?: string;
  readonly errors?: Record<number, string>;
};

export class ApiError extends Error {
  public readonly url: string;
  public readonly status: number;
  public readonly statusText: string;
  public readonly body: any;
  public readonly request: RequestOptions;

  constructor(request: RequestOptions, response: ApiResult, message: string) {
    super(message);

    this.name = 'ApiError';
    this.url = response.url;
    this.status = response.status;
    this.statusText = response.statusText;
    this.body = response.body;
    this.request = request;
  }
}

export type ApiResult = {
  readonly url: string;
  readonly ok: boolean;
  readonly status: number;
  readonly statusText: string;
  readonly body: any;
};

const isDefined = <T>(value: T | null | undefined): value is Exclude<T, null | undefined> => {
  return value !== undefined && value !== null;
};

const isString = (value: any): value is string => {
  return typeof value === 'string';
};

const isStringWithValue = (value: any): value is string => {
  return isString(value) && value !== '';
};

const isBlob = (value: any): value is Blob => {
  return (
    typeof value === 'object' &&
    typeof value.type === 'string' &&
    typeof value.stream === 'function' &&
    typeof value.arrayBuffer === 'function' &&
    typeof value.constructor === 'function' &&
    typeof value.constructor.name === 'string' &&
    /^(Blob|File)$/.test(value.constructor.name) &&
    /^(Blob|File)$/.test(value[Symbol.toStringTag])
  );
};

const isFormData = (value: any): value is FormData => {
  return value instanceof FormData;
};

const base64 = (str: string): string => {
  try {
    return btoa(str);
  } catch (err) {
    // @ts-ignore
    return Buffer.from(str).toString('base64');
  }
};

const getQueryString = (params: Record<string, any>): string => {
  const qs: string[] = [];

  const append = (key: string, value: any) => {
    qs.push(`${encodeURIComponent(key)}=${encodeURIComponent(String(value))}`);
  };

  const process = (key: string, value: any) => {
    if (isDefined(value)) {
      if (Array.isArray(value)) {
        value.forEach(v => {
          process(key, v);
        });
      } else if (typeof value === 'object') {
        Object.entries(value).forEach(([k, v]) => {
          process(`${key}[${k}]`, v);
        });
      } else {
        append(key, value);
      }
    }
  };

  Object.entries(params).forEach(([key, value]) => {
    process(key, value);
  });

  if (qs.length > 0) {
    return `?${qs.join('&')}`;
  }

  return '';
};

const getUrl = (config: ServerConfig, options: RequestOptions): string => {
  const path = options.url.replace(/{(.*?)}/g, (substring: string, group: string) => {
    if (options.path?.hasOwnProperty(group)) {
      return String(options.path[group]);
    }
    return substring;
  });

  const url = `${config.url}${path}`;
  if (options.query) {
    return `${url}${getQueryString(options.query)}`;
  }
  return url;
};

const getFormData = (options: RequestOptions): FormData | undefined => {
  if (options.formData) {
    const formData = new FormData();

    const process = (key: string, value: any) => {
      if (isString(value) || isBlob(value)) {
        formData.append(key, value);
      } else {
        formData.append(key, JSON.stringify(value));
      }
    };

    Object.entries(options.formData)
      .filter(([_, value]) => isDefined(value))
      .forEach(([key, value]) => {
        if (Array.isArray(value)) {
          value.forEach(v => process(key, v));
        } else {
          process(key, value);
        }
      });

    return formData;
  }
  return undefined;
};

type Resolver<T> = (options: RequestOptions) => Promise<T>;

const resolve = async <T>(
  options: RequestOptions,
  resolver?: T | Resolver<T>
): Promise<T | undefined> => {
  if (typeof resolver === 'function') {
    return (resolver as Resolver<T>)(options);
  }
  return resolver;
};

const getHeaders = async (config: ServerConfig, options: RequestOptions): Promise<Headers> => {
  const headers = Object.entries({
    Accept: 'application/json',
    ...options.headers,
  })
    .filter(([_, value]) => isDefined(value))
    .reduce(
      (headers, [key, value]) => ({
        ...headers,
        [key]: String(value),
      }),
      {} as Record<string, string>
    );

  if (options.body) {
    if (options.mediaType) {
      headers['Content-Type'] = options.mediaType;
    } else if (isBlob(options.body)) {
      headers['Content-Type'] = options.body.type || 'application/octet-stream';
    } else if (isString(options.body)) {
      headers['Content-Type'] = 'text/plain';
    } else if (!isFormData(options.body)) {
      headers['Content-Type'] = 'application/json';
    }
  }

  return new Headers(headers);
};

const getRequestBody = (options: RequestOptions): any => {
  if (options.body) {
    if (options.mediaType?.includes('/json')) {
      return JSON.stringify(options.body);
    } else if (isString(options.body) || isBlob(options.body) || isFormData(options.body)) {
      return options.body;
    } else {
      return JSON.stringify(options.body);
    }
  }
  return undefined;
};

export const sendRequest = async (
  config: ServerConfig,
  options: RequestOptions,
  url: string,
  body: any,
  formData: FormData | undefined,
  headers: Headers,
  onCancel: OnCancel
): Promise<Response> => {
  const controller = new AbortController();

  const request: RequestInit = {
    headers,
    body: body ?? formData,
    method: options.method,
    signal: controller.signal,
  };

  onCancel(() => controller.abort());

  return await fetch(url, request);
};

const getResponseHeader = (response: Response, responseHeader?: string): string | undefined => {
  if (responseHeader) {
    const content = response.headers.get(responseHeader);
    if (isString(content)) {
      return content;
    }
  }
  return undefined;
};

const getResponseBody = async (response: Response): Promise<any> => {
  if (response.status !== 204) {
    try {
      const contentType = response.headers.get('Content-Type');
      if (contentType) {
        const isJSON = contentType.toLowerCase().startsWith('application/json');
        if (isJSON) {
          return await response.json();
        } else {
          return await response.text();
        }
      }
    } catch (error) {
      console.error(error);
    }
  }
  return undefined;
};

const catchErrorCodes = (options: RequestOptions, result: ApiResult): void => {
  const errors: Record<number, string> = {
    400: 'Bad Request',
    401: 'Unauthorized',
    403: 'Forbidden',
    404: 'Not Found',
    500: 'Internal Server Error',
    502: 'Bad Gateway',
    503: 'Service Unavailable',
    ...options.errors,
  };

  const error = errors[result.status];
  if (error) {
    throw new ApiError(options, result, error);
  }

  if (!result.ok) {
    throw new ApiError(options, result, 'Generic Error');
  }
};

/**
 * Request method
 * @param config The OpenAPI configuration object
 * @param options The request options from the service
 * @returns CancelablePromise<T>
 * @throws ApiError
 */
export const request = <T>(config: ServerConfig, options: RequestOptions): CancelablePromise<T> => {
  return new CancelablePromise(async (resolve, reject, onCancel) => {
    try {
      const url = getUrl(config, options);
      const formData = getFormData(options);
      const body = getRequestBody(options);
      const headers = await getHeaders(config, options);

      if (!onCancel.isCancelled) {
        const response = await sendRequest(config, options, url, body, formData, headers, onCancel);
        const responseBody = await getResponseBody(response);
        const responseHeader = getResponseHeader(response, options.responseHeader);

        const result: ApiResult = {
          url,
          ok: response.ok,
          status: response.status,
          statusText: response.statusText,
          body: responseHeader ?? responseBody,
        };

        catchErrorCodes(options, result);

        resolve(result.body);
      }
    } catch (error) {
      reject(error);
    }
  });
};

/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export class CancelError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'CancelError';
  }

  public get isCancelled(): boolean {
    return true;
  }
}

export interface OnCancel {
  readonly isResolved: boolean;
  readonly isRejected: boolean;
  readonly isCancelled: boolean;

  (cancelHandler: () => void): void;
}

export class CancelablePromise<T> implements Promise<T> {
  readonly [Symbol.toStringTag]!: string;

  private _isResolved: boolean;
  private _isRejected: boolean;
  private _isCancelled: boolean;
  private readonly _cancelHandlers: (() => void)[];
  private readonly _promise: Promise<T>;
  private _resolve?: (value: T | PromiseLike<T>) => void;
  private _reject?: (reason?: any) => void;

  constructor(
    executor: (
      resolve: (value: T | PromiseLike<T>) => void,
      reject: (reason?: any) => void,
      onCancel: OnCancel
    ) => void
  ) {
    this._isResolved = false;
    this._isRejected = false;
    this._isCancelled = false;
    this._cancelHandlers = [];
    this._promise = new Promise<T>((resolve, reject) => {
      this._resolve = resolve;
      this._reject = reject;

      const onResolve = (value: T | PromiseLike<T>): void => {
        if (this._isResolved || this._isRejected || this._isCancelled) {
          return;
        }
        this._isResolved = true;
        this._resolve?.(value);
      };

      const onReject = (reason?: any): void => {
        if (this._isResolved || this._isRejected || this._isCancelled) {
          return;
        }
        this._isRejected = true;
        this._reject?.(reason);
      };

      const onCancel = (cancelHandler: () => void): void => {
        if (this._isResolved || this._isRejected || this._isCancelled) {
          return;
        }
        this._cancelHandlers.push(cancelHandler);
      };

      Object.defineProperty(onCancel, 'isResolved', {
        get: (): boolean => this._isResolved,
      });

      Object.defineProperty(onCancel, 'isRejected', {
        get: (): boolean => this._isRejected,
      });

      Object.defineProperty(onCancel, 'isCancelled', {
        get: (): boolean => this._isCancelled,
      });

      return executor(onResolve, onReject, onCancel as OnCancel);
    });
  }

  public then<TResult1 = T, TResult2 = never>(
    onFulfilled?: ((value: T) => TResult1 | PromiseLike<TResult1>) | null,
    onRejected?: ((reason: any) => TResult2 | PromiseLike<TResult2>) | null
  ): Promise<TResult1 | TResult2> {
    return this._promise.then(onFulfilled, onRejected);
  }

  public catch<TResult = never>(
    onRejected?: ((reason: any) => TResult | PromiseLike<TResult>) | null
  ): Promise<T | TResult> {
    return this._promise.catch(onRejected);
  }

  public finally(onFinally?: (() => void) | null): Promise<T> {
    return this._promise.finally(onFinally);
  }

  public cancel(): void {
    if (this._isResolved || this._isRejected || this._isCancelled) {
      return;
    }
    this._isCancelled = true;
    if (this._cancelHandlers.length) {
      try {
        for (const cancelHandler of this._cancelHandlers) {
          cancelHandler();
        }
      } catch (error) {
        console.warn('Cancellation threw an error', error);
        return;
      }
    }
    this._cancelHandlers.length = 0;
    this._reject?.(new CancelError('Request aborted'));
  }

  public get isCancelled(): boolean {
    return this._isCancelled;
  }
}
