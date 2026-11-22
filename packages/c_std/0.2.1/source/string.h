// Fixed-width C ABI declarations for supported 64-bit targets. No includes.
void *memcpy(void *destination, const void *source, uint64_t count);
void *memmove(void *destination, const void *source, uint64_t count);
void *memset(void *destination, int32_t value, uint64_t count);
int32_t memcmp(const void *a, const void *b, uint64_t count);
void *memchr(const void *memory, int32_t character, uint64_t count);
uint64_t strlen(const int8_t *text);
int8_t *strcpy(int8_t *destination, const int8_t *source);
int8_t *strncpy(int8_t *destination, const int8_t *source, uint64_t count);
int8_t *strcat(int8_t *destination, const int8_t *source);
int8_t *strncat(int8_t *destination, const int8_t *source, uint64_t count);
int32_t strncmp(const int8_t *a, const int8_t *b, uint64_t count);
int32_t strcoll(const int8_t *a, const int8_t *b);
uint64_t strxfrm(int8_t *destination, const int8_t *source, uint64_t count);
int8_t *strchr(const int8_t *text, int32_t character);
int8_t *strrchr(const int8_t *text, int32_t character);
uint64_t strspn(const int8_t *text, const int8_t *accepted);
uint64_t strcspn(const int8_t *text, const int8_t *rejected);
int8_t *strpbrk(const int8_t *text, const int8_t *accepted);
int8_t *strstr(const int8_t *text, const int8_t *needle);
int8_t *strtok(int8_t *text, const int8_t *delimiters);
int8_t *strerror(int32_t error);

int32_t strcmp(const int8_t *a, const int8_t *b);
