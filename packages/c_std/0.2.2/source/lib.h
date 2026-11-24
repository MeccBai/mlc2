// Fixed-width C ABI declarations for supported 64-bit targets. No includes.
int32_t atoi(const int8_t *text);
int64_t atoll(const int8_t *text);
double atof(const int8_t *text);
float strtof(const int8_t *text, int8_t **end);
double strtod(const int8_t *text, int8_t **end);
int64_t strtoll(const int8_t *text, int8_t **end, int32_t base);
uint64_t strtoull(const int8_t *text, int8_t **end, int32_t base);
void *malloc(uint64_t size);
void *calloc(uint64_t count, uint64_t size);
void *realloc(void *memory, uint64_t size);
void free(void *memory);
int32_t rand(void);
void srand(uint32_t seed);
void exit(int32_t status);
int32_t system(const int8_t *command);
int8_t *getenv(const int8_t *name);
int64_t llabs(int64_t value);

int32_t abs(int32_t value);
void abort(void);
