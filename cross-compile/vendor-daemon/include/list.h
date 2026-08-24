#ifndef _LINUX_LIST_H_
#define _LINUX_LIST_H_

#ifdef __cplusplus
extern "C" {
#endif

struct list_head {
	struct list_head *next, *prev;
};

static inline void INIT_LIST_HEAD(struct list_head *list)
{
	list->next = list;
	list->prev = list;
}

#ifdef __cplusplus
}
#endif

#endif /* _LINUX_LIST_H_ */
