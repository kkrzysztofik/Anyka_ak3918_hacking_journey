/**
 * N1 enumeration data definitions.
 *
 */

#include <NkUtils/types.h>

#ifndef NK_ENUM_H_
#define NK_ENUM_H_
NK_CPP_EXTERN_BEGIN

/**
 * Undefined enumeration variable.
 */
#define NK_ENUM_UNDEFINED ""

/**
 * Interface template for defining enum-value-to-text mappings.
 * Using this interface to define functions effectively resolves duplicate naming issues.
 * The macro expands into the corresponding typed interface at compile time.
 */
#define DECLARE_NK_ENUM_MAP(__type)  NK_PChar NK_Enum_Map##__type(NK_##__type enm)

/**
 * Interface template for mapping text to enum values.
 */
#define DECLARE_NK_ENUM_UNMAP(__type)  NK_##__type NK_Enum_Unmap##__type(NK_PChar text)


/**
 * Call the corresponding typed enum interface.
 */
#define NK_ENUM_MAP(__type, __enm) NK_Enum_Map##__type(__enm)

/**
 * Call the corresponding typed enum interface.
 */
#define NK_ENUM_UNMAP(__type, __text) NK_Enum_Unmap##__type(__text)



NK_CPP_EXTERN_END
#endif /* NK_N1_ENUM_H_ */
