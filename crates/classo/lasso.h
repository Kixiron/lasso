#ifndef LASSO_H
#define LASSO_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>


/**
 * A memory allocation failed
 */
#define LASSO_ERROR_FAILED_ALLOCATION 3

/**
 * A [`Key`] implementation returned [`None`], meaning it could not produce any more keys
 */
#define LASSO_ERROR_KEY_SPACE_EXHAUSTION 2

/**
 * A memory limit set using [`MemoryLimits`] was reached, and no more memory could be allocated
 *
 * [`MemoryLimits`]: lasso::MemoryLimits
 */
#define LASSO_ERROR_MEMORY_LIMIT_REACHED 1

/**
 * No error occurred
 */
#define LASSO_ERROR_NONE 0

/**
 * The value of an invalid `key`, used to tell if a function has returned
 * a key or nothing
 *
 * ```c
 * #include <stdio.h>
 * #include <string.h>
 * #include "lasso.h"
 *
 * int main() {
 *     Rodeo *rodeo = lasso_rodeo_new();
 *
 *     const char *string = "un-interned string!";
 *     uint32_t key = lasso_rodeo_get(rodeo, string, strlen(string));
 *     if (key == LASSO_INVALID_KEY) {
 *         printf("%s has no key!\n", string);
 *     } else {
 *         printf("%s has the key %u\n", key);
 *     }
 * }
 * ```
 */
#define LASSO_INVALID_KEY 0

/**
 * An error that's safe to pass across the FFI boundary
 */
enum LassoErrorKind
#ifdef __cplusplus
  : uint8_t
#endif // __cplusplus
 {
  /**
   * No error occurred
   */
  None = 0,
  /**
   * A memory limit set using [`MemoryLimits`] was reached, and no more memory could be allocated
   *
   * [`MemoryLimits`]: lasso::MemoryLimits
   */
  MemoryLimitReached = 1,
  /**
   * A [`Key`] implementation returned [`None`], meaning it could not produce any more keys
   */
  KeySpaceExhaustion = 2,
  /**
   * A memory allocation failed
   */
  FailedAllocation = 3,
};
#ifndef __cplusplus
typedef uint8_t LassoErrorKind;
#endif // __cplusplus

/**
 * A shim struct to allow passing [`Rodeo`]s across the FFI boundary
 * as opaque pointers
 */
typedef struct Rodeo Rodeo;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Dispose of a previously created [`Rodeo`]
 *
 * # Safety
 *
 * - `rodeo` must be non-null and must have previously been created by [`lasso_rodeo_new()`]
 *   and must not yet have been disposed of
 *
 * ```c
 * #include "lasso.h"
 *
 * int main() {
 *     Rodeo *rodeo = lasso_rodeo_new();
 *
 *     // Dispose of the interner
 *     lasso_rodeo_dispose(rodeo);
 * }
 * ```
 *
 */
void lasso_rodeo_dispose(struct Rodeo *rodeo);

/**
 * Get the key value of a string, returning [`LASSO_INVALID_KEY`] if it doesn't exist
 *
 * # Safety
 *
 * - `rodeo` must be non-null and not currently mutably borrowed.
 * - `string` must be non-null and point to an array of utf8-encoded bytes of `length` length.
 *
 * ```c
 * #include <assert.h>
 * #include <stdint.h>
 * #include "lasso.h"
 *
 * int main() {
 *     Rodeo *rodeo = lasso_rodeo_new();
 *
 *     char *some_random_string = "Any string you can imagine!";
 *
 *     // Attempt to get the key for a string
 *     uint32_t key = lasso_rodeo_get(
 *         rodeo,
 *         some_random_string,
 *         strlen(some_random_string),
 *     );
 *
 *     // `lasso_rodeo_get()` returns `LASSO_INVALID_KEY` if the string hasn't
 *     // been interned yet, so this call will succeed since the string we tried
 *     // to get hasn't been interned
 *     assert(key == LASSO_INVALID_KEY);
 *
 *     lasso_rodeo_dispose(rodeo);
 * }
 * ```
 *
 * ```c
 * #include <assert.h>
 * #include <stdint.h>
 * #include "lasso.h"
 *
 * int main() {
 *     Rodeo *rodeo = lasso_rodeo_new();
 *
 *     char *some_random_string = "Any string you can imagine!";
 *
 *     // Intern the string so the `lasso_rodeo_get()` call succeeds
 *     LassoErrorKind error = LASSO_ERROR_NONE;
 *     lasso_rodeo_get_or_intern(
 *         rodeo,
 *         some_random_string,
 *         strlen(some_random_string),
 *         &error,
 *     );
 *     assert(error == LASSO_ERROR_NONE);
 *
 *     // Attempt to get the key for a string
 *     uint32_t key = lasso_rodeo_get(
 *         rodeo,
 *         some_random_string,
 *         strlen(some_random_string),
 *     );
 *
 *     // `lasso_rodeo_get()` returns `LASSO_INVALID_KEY` if the string hasn't
 *     // been interned yet, so this call will succeed since the string we tried
 *     // to get has been interned
 *     assert(key != LASSO_INVALID_KEY);
 *
 *     lasso_rodeo_dispose(rodeo);
 * }
 * ```
 *
 */
uint32_t lasso_rodeo_get(struct Rodeo *rodeo, char *string, uint64_t length);

/**
 * Get a string if it's been previously interned or intern it if it does not yet exist
 *
 * If an error occurs while attempting to intern a string then [`LASSO_INVALID_KEY`]
 * will be returned and the pointed-to value within `error` will be populated with
 * the error code as a [`FFILassoErrorKind`]. If no error occurs then the pointed-to
 * value will be populated with [`FFILassoErrorKind::None`] and the returned value
 * will be the key associated with the given string
 *
 * # Safety
 *
 * - `rodeo` must be non-null and not currently mutably borrowed.
 * - `string` must be non-null and point to an array of utf8-encoded bytes of `length` length.
 * - `error` must be a non-null pointer to a `u8`
 *
 * ```c
 * #include <assert.h>
 * #include <stdint.h>
 * #include "lasso.h"
 *
 * int main() {
 *     Rodeo *rodeo = lasso_rodeo_new();
 *
 *     char *some_random_string = "Any string you can imagine!";
 *
 *     // Create a variable to hold the error if one occurs
 *     LassoErrorKind error = LASSO_ERROR_NONE;
 *
 *     // Intern the string if it doesn't exist and return the key assigned to it
 *     uint32_t key = lasso_rodeo_get_or_intern(
 *         rodeo,
 *         some_random_string,
 *         strlen(some_random_string),
 *         &error,
 *     );
 *
 *     // If everything went smoothly, the returned `key` will have the key associated
 *     // with the given string and `error` will hold `LASSO_ERROR_NONE`
 *     if (error == LASSO_ERROR_NONE) {
 *         assert(key != LASSO_INVALID_KEY);
 *     }
 *     // If the intern call succeeds, `error` will hold the error that occurred
 *     // and the returned key will be `LASSO_INVALID_KEY`
 *     else {
 *         assert(key == LASSO_INVALID_KEY);
 *     }
 *
 *     lasso_rodeo_dispose(rodeo);
 * }
 * ```
 *
 */
uint32_t lasso_rodeo_get_or_intern(struct Rodeo *rodeo,
                                   char *string,
                                   uint64_t length,
                                   LassoErrorKind *error);

/**
 * Create a new [`Rodeo`] with the default settings
 *
 * Uses a [`Spur`] (`uint32_t`) as the key type and Rust's [`RandomState`] as the underlying hasher
 *
 * ```c
 * #include "lasso.h"
 *
 * int main() {
 *     // Create a new interner
 *     Rodeo *rodeo = lasso_rodeo_new();
 *
 *     lasso_rodeo_dispose(rodeo);
 * }
 * ```
 *
 * [`RandomState`]: std::collections::hash_map::RandomState
 */
struct Rodeo *lasso_rodeo_new(void);

/**
 * Resolve a key into a string, returning an [`None`] if one occurs
 *
 * `length` will be set to the length of the string in utf8 encoded bytes
 * and the return value will be null if the key does not exist in the current
 * interner
 *
 * # Safety
 *
 * - `rodeo` must be non-null and not currently mutably borrowed.
 *
 * ```c
 * #include <assert.h>
 * #include <stdio.h>
 * #include <stdint.h>
 * #include "lasso.h"
 *
 * int main() {
 *     Rodeo *rodeo = lasso_rodeo_new();
 *
 *     char *some_random_string = "Any string you can imagine!";
 *
 *     // Intern the string
 *     LassoErrorKind error = LASSO_ERROR_NONE;
 *     uint32_t key = lasso_rodeo_get_or_intern(
 *         rodeo,
 *         some_random_string,
 *         strlen(some_random_string),
 *         &error,
 *     );
 *     assert(key != LASSO_INVALID_KEY);
 *
 *     // Make a variable to hold the string's length
 *     uint64_t length = 0;
 *
 *     // Resolve the key for the underlying string
 *     char *ptr = lasso_rodeo_resolve(rodeo, key, &length);
 *
 *     // If the key is found in the current interner, `ptr` will hold the pointer to
 *     // the string and `length` will hold the length of the string in utf8 encoded
 *     // bytes
 *     if (ptr) {
 *         printf("The key %u resolved to the string ", key);
 *         // We use `fwrite()` to print out the string since utf8 allows interior null bytes
 *         fwrite(string, sizeof(char), length, stdout);
 *         printf("\n");
 *     }
 *     // If the key doesn't exist in the current interner, `ptr` will be null
 *     else {
 *         printf("The key %u could not be found!\n", key);
 *     }
 *
 *     lasso_rodeo_dispose(rodeo);
 * }
 * ```
 *
 */
char *lasso_rodeo_resolve(struct Rodeo *rodeo, uint32_t key, uint64_t *length);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* LASSO_H */
