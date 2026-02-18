/**
 * JPEG processing related operation methods.
 */

#include <akae_typedef.h>
#include <akae_log.h>


#if !defined(AKAE_JPEG_H__)
#define AKAE_JPEG_H__
AK_C_HEADER_EXTERN_C_BEGIN


#define AK_JPEG_SEG_MAKER   (0xFF)	//none	Start of Image

#define AK_JPEG_SEG_SOI     (0xD8)	//none	Start of Image
#define AK_JPEG_SEG_S0F(__n) \
	(0xC0 + (__n))	                ///< Start of Frame, variable size.

#define AK_JPEG_SEG_DHT     (AK_JPEG_SEG_S0F (4))	/// Define Huffman Tables, variable size
#define AK_JPEG_SEG_DQT     (0xDB)	///< Define Quantization Table(s), variable size
#define AK_JPEG_SEG_DRI     (0xDD)	//4 bytes	Define Restart Interval
#define AK_JPEG_SEG_SOS     (0xDA)	//variable size	Start Of Scan
//#define AK_JPEG_SEG_RSTn    (0xD )  ////n//(//n//#0..7)	none	Restart
#define AK_JPEG_SEG_APP(__n) \
	(0xE0 + (__n))					///< Application specific
#define AK_JPEG_SEG_COM     (0xFE)	//variable size	Comment
#define AK_JPEG_SEG_EOI     (0xD9)	//none	End Of Image


#pragma pack(push, 1)

/**
 * JPEG file segment header data structure.
 */
typedef struct _AK_JpegSegmentHeader {

	/// FF marker, using 0xff as segment boundary.
	AK_uint8 ff;

	/// Segment name, such as @ref AK_JPEG_SEG_SOI.
	AK_uint8 name;

	/// Segment data length, in bytes.
	AK_uint16 length;

} AK_JpegSegmentHeader;


/**
 * JPEG SOF0 data structure.
 */
typedef struct _AK_JpegSegmentSOF0 {

	AK_JpegSegmentHeader Header;

	/// This is in bits/sample, usually 8 (12 and 16 not supported by most software).
	AK_uint8   precision;

	/// This must be > 0
	AK_uint16  height;

	/// This must be > 0
	AK_uint16  width;

	/// Usually 1 = grey scaled, 3 = color YcbCr or YIQ 4 = color CMYK
	AK_byte    ncomponent;

	/// Read each component data of 3 bytes. It contains,
	/// (component Id(1byte)(1 = Y, 2 = Cb, 3 = Cr, 4 = I, 5 = Q),
	/// sampling factors (1byte) (bit 0-3 vertical., 4-7 horizontal.),
	/// quantization table number (1 byte)).
	/// 4 components maxmam.
	AK_byte    components[4][3];

} AK_JpegSegmentSOF0;

#pragma pack(pop)


typedef struct _AK_JpegParseFile {

	struct {

		/// Offset position of the attribute.
		AK_bytptr offset;

		/// Memory length occupied by the attribute.
		AK_size len;

	} SOI, EOI, SOF[8], SOF2, DHT, DQT[2], SOS, RST[8], APP[8], COM, Scans;

	/// Image width, in pixels.
	AK_size width;

	/// Image height, in pixels.
	AK_size height;

	/// Lumi Q Table
	AK_byte lqt[64];

	/// Chroma Q Table
	AK_byte cqt[64];

} AK_JpegParseFile;



/**
 *
 *
 */
AK_API AK_int akae_jpeg_parse_file (AK_bytptr data, AK_size datalen, AK_JpegParseFile *Parse);


AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_JPEG_H__
