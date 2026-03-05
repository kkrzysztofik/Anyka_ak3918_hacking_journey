
/**
 * Object definition module.
 * This module defines basic object types for object inheritance and polymorphism implementation.
 */

#include <akae_stdlib.h>

#if !defined(AKSPC_OBJECT_H_)
#define AKSPC_OBJECT_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Object structure.
 * Object initialization interface corresponds to create and release methods.
 */
typedef struct  _AK_Object {
#define AK_This struct _AK_Object *const

	/// Object name.
	AK_chrcptr   (*name) (AK_This, AK_chrptr stack, AK_size stacklen);

	/// Object print output.
	AK_void     (*dump) (AK_This);

	/// Object thread-safety flag.
	AK_boolean  (*threadsafe) (AK_This);

	/// Release object implementation.
	AK_void     (*release) (AK_This);

#undef AK_This
} *AK_Object;



/**
 * Static object structure.
 * Static object initialization interface corresponds to init and destroy methods.
 */
typedef struct  _AK_StaticObject {

	/// Not visible to the user.
	AK_byte _opacity[sizeof(struct _AK_Object) + 256];

} AK_StaticObject;


AK_C_HEADER_EXTERN_C_END
#endif ///< OBJECT_H_
