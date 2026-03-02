/**************************************************************************/
/* LabWindows/CVI User Interface Resource (UIR) Include File              */
/*                                                                        */
/* WARNING: Do not add to, delete from, or otherwise modify the contents  */
/*          of this include file.                                         */
/**************************************************************************/

#include <userint.h>

#ifdef __cplusplus
    extern "C" {
#endif

     /* Panels and Controls: */

#define  PANEL                            1
#define  PANEL_LISTBOX                    2       /* control type: listBox, callback function: listbox_cb */
#define  PANEL_QUITBUTTON                 3       /* control type: command, callback function: quit_cb */
#define  PANEL_Y_OFFSET                   4       /* control type: numeric, callback function: (none) */
#define  PANEL_X_OFFSET                   5       /* control type: numeric, callback function: (none) */
#define  PANEL_SCALE                      6       /* control type: numeric, callback function: (none) */
#define  PANEL_ROTATION                   7       /* control type: numeric, callback function: (none) */
#define  PANEL_TEXTMSG_3                  8       /* control type: textMsg, callback function: (none) */
#define  PANEL_IMAGECTRL_ROT              9       /* control type: deco, callback function: (none) */
#define  PANEL_TEXTMSG_2                  10      /* control type: textMsg, callback function: (none) */
#define  PANEL_TEXTMSG_4                  11      /* control type: textMsg, callback function: (none) */
#define  PANEL_IMAGECTRL_REG              12      /* control type: deco, callback function: (none) */
#define  PANEL_IMAGECTRL_REF              13      /* control type: deco, callback function: (none) */
#define  PANEL_TEXTMSG                    14      /* control type: textMsg, callback function: (none) */
#define  PANEL_PICTURE                    15      /* control type: picture, callback function: (none) */


     /* Control Arrays: */

          /* (no control arrays in the resource file) */


     /* Menu Bars, Menus, and Menu Items: */

          /* (no menu bars in the resource file) */


     /* Callback Prototypes: */

int  CVICALLBACK listbox_cb(int panel, int control, int event, void *callbackData, int eventData1, int eventData2);
int  CVICALLBACK quit_cb(int panel, int control, int event, void *callbackData, int eventData1, int eventData2);


#ifdef __cplusplus
    }
#endif