// Compiler FFI declarations: explicit-width types, no preprocessing.
void abort(void);
int32_t abs(int32_t value);
int32_t _ecvt_s(int8_t *buffer, uint64_t capacity, double value, int32_t count, int32_t *decimal, int32_t *sign);
void *__acrt_iob_func(uint32_t index);
uint64_t fwrite(const void *buffer, uint64_t size, uint64_t count, void *stream);
int32_t snprintf(int8_t *buffer, uint64_t capacity, const int8_t *pattern, ...);
int32_t strcmp(const int8_t *a, const int8_t *b);
