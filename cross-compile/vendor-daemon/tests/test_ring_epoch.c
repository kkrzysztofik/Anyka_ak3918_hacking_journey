/* Host-compiled unit test for ring-header epoch generation. */
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include "vd_ring_buffer.h"

int main(void)
{
    void *base_a;
    void *base_b;
    struct vd_ring_header *hdr;
    uint32_t epoch_a;
    uint32_t epoch_b;

    /* Fresh ring: epoch must be non-zero (0 is reserved for "detached"). */
    base_a = vd_ring_create();
    assert(base_a != NULL);
    hdr = vd_ring_get_header(base_a);
    assert(hdr->magic == VD_SHM_MAGIC);
    assert(hdr->version == VD_SHM_VERSION);
    epoch_a = hdr->epoch;
    assert(epoch_a != 0);

    /* Re-create over the SAME file, as a daemon restart does. The epoch must
     * change, because that is the only evidence of the restart the client has. */
    base_b = vd_ring_create();
    assert(base_b != NULL);
    hdr = vd_ring_get_header(base_b);
    epoch_b = hdr->epoch;
    assert(epoch_b != 0);
    assert(epoch_b != epoch_a);

    /* Header must still be exactly 64 bytes. */
    assert(sizeof(struct vd_ring_header) == 64);

    unlink(VD_SHM_PATH);
    printf("test_ring_epoch: PASS\n");
    return 0;
}
