// Windows LLP64: C long is 32 bits; these APIs must not be universal.
int32_t atol(const int8_t *text);
int32_t labs(int32_t value);
int32_t strtol(const int8_t *text, int8_t **end, int32_t base);
uint32_t strtoul(const int8_t *text, int8_t **end, int32_t base);
