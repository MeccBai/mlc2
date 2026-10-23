// Windows UCRT uses explicit 64-bit time entry points and a 32-bit clock_t.
struct tm {
    int32_t tm_sec; int32_t tm_min; int32_t tm_hour;
    int32_t tm_mday; int32_t tm_mon; int32_t tm_year;
    int32_t tm_wday; int32_t tm_yday; int32_t tm_isdst;
};
int64_t _time64(int64_t *result);
double _difftime64(int64_t end, int64_t beginning);
int64_t _mktime64(struct tm *value);
int32_t _gmtime64_s(struct tm *result, const int64_t *time);
int32_t _localtime64_s(struct tm *result, const int64_t *time);
int32_t _ctime64_s(int8_t *buffer, uint64_t capacity, const int64_t *time);
int32_t asctime_s(int8_t *buffer, uint64_t capacity, const struct tm *time);
uint64_t strftime(int8_t *buffer, uint64_t capacity, const int8_t *pattern, const struct tm *time);
int32_t clock(void);
