
/**
 * Module base data structure.\n
 * Serves as the parent class for similar modules.\n
 * Forced cast of a subclass module data structure yields the subclass module name.
 *
 * NK_Object content can be accessed via polymorphism.
 *
 */

#include "types.h"


#ifndef NK_OBJECT_H_
#define NK_OBJECT_H_
NK_CPP_EXTERN_BEGIN

/**
 * Object module handle.
 */
typedef struct Nk_Object
{
#define THIS struct Nk_Object *const

	/**
	 * Get this module's name.
	 */
	NK_PChar
	(*name)(THIS);

	/**
	 * Debug-print module-related information.
	 */
	NK_Void
	(*dump)(THIS);

	/**
	 * Thread-safety flag declaration.\n
	 * Indicates whether this module's operation interfaces are thread-safe.
	 */
	NK_Boolean
	(*threadsafe)(THIS);

	/**
	 * Release the current module.\n
	 * After calling this interface, the module handle is no longer valid;\n
	 * the caller must promptly set the handle to Nil to avoid unnecessary errors.
	 */
	NK_Void
	(*free)(THIS);

#undef THIS
} NK_Object;


NK_CPP_EXTERN_END
#endif /* NK_OBJECT_H_*/
