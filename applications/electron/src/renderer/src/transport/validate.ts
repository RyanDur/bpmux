import { type Decoder, type DecoderError } from 'schemawax'
import { asyncFailure, asyncSuccess, type AsyncResult } from '../lib/asyncResult'
import { type HttpError } from './http'

const describeDecoderError = (error: DecoderError): string =>
  error.path.length > 0 ? `${error.message} (at ${error.path.join(' > ')})` : error.message

export const validate =
  <T>(decoder: Decoder<T>) =>
  (raw: unknown): AsyncResult<T, HttpError> => {
    const result = decoder.validate(raw)
    return result.type === 'ok'
      ? asyncSuccess<T, HttpError>(result.data)
      : asyncFailure<HttpError, T>({ kind: 'decode', message: describeDecoderError(result.error) })
  }
