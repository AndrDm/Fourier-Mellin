#pragma pack(4)
typedef struct {char *name; void *address; unsigned long isFunction:1; unsigned long reserved:31;} ExeSymbol;
int __cdecl listbox_cb (int panel, int control, int event, void *callbackData, int eventData1, int eventData2);
int __cdecl quit_cb (int panel, int control, int event, void *callbackData, int eventData1, int eventData2);
int __UICallbackSymbolCount = 2;
ExeSymbol __UICallbackSymbols [2] =
{
 {"listbox_cb", (void*)listbox_cb, 1, 0},
 {"quit_cb", (void*)quit_cb, 1, 0}
};