// Windows LLP64 file offsets: long is i32, fpos_t and explicit large offsets are i64.
int32_t fseek(void *stream, int32_t offset, int32_t origin);
int32_t ftell(void *stream);
int32_t _fseeki64(void *stream, int64_t offset, int32_t origin);
int64_t _ftelli64(void *stream);
int32_t fgetpos(void *stream, int64_t *position);
int32_t fsetpos(void *stream, const int64_t *position);
