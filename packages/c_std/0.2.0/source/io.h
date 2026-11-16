// Fixed-width C ABI declarations for supported 64-bit targets. No includes.
// FILE and fpos_t have no portable layout. FILE is passed as an opaque void pointer.
void *fopen(const int8_t *filename, const int8_t *mode);
void *freopen(const int8_t *filename, const int8_t *mode, void *stream);
int32_t fclose(void *stream);
int32_t fflush(void *stream);
void setbuf(void *stream, int8_t *buffer);
int32_t setvbuf(void *stream, int8_t *buffer, int32_t mode, uint64_t size);
int32_t fgetc(void *stream);
int8_t *fgets(int8_t *buffer, int32_t count, void *stream);
int32_t fputc(int32_t character, void *stream);
int32_t fputs(const int8_t *text, void *stream);
int32_t ungetc(int32_t character, void *stream);
uint64_t fread(void *buffer, uint64_t size, uint64_t count, void *stream);
int32_t fprintf(void *stream, const int8_t *pattern, ...);
int32_t fscanf(void *stream, const int8_t *pattern, ...);
int32_t sprintf(int8_t *buffer, const int8_t *pattern, ...);
int32_t sscanf(const int8_t *buffer, const int8_t *pattern, ...);
void rewind(void *stream);
void clearerr(void *stream);
int32_t feof(void *stream);
int32_t ferror(void *stream);
void perror(const int8_t *message);
int32_t remove(const int8_t *filename);
int32_t rename(const int8_t *old_name, const int8_t *new_name);
void *tmpfile(void);
int8_t *tmpnam(int8_t *buffer);

int32_t printf(const int8_t *pattern, ...);
int32_t scanf(const int8_t *pattern, ...);
int32_t puts(const int8_t *text);
int32_t putchar(int32_t character);
int32_t getchar(void);
uint64_t fwrite(const void *buffer, uint64_t size, uint64_t count, void *stream);
int32_t snprintf(int8_t *buffer, uint64_t capacity, const int8_t *pattern, ...);
