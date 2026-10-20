/*
 * Cmd+C / Cmd+V sintéticos en macOS, hablando directo con CoreGraphics.
 *
 * Antes se usaba `enigo`, que manda "Cmd abajo", luego "c" y luego "Cmd
 * arriba" como eventos sueltos, SIN marcar Cmd en el evento de la "c": confía
 * en que el WindowServer recuerde que Cmd está pulsado. Si el usuario todavía
 * tiene apretadas (o suelta justo en ese momento) las teclas del atajo, ese
 * estado global cambia a mitad de la secuencia y la "c" llega sola: la app la
 * escribe encima de la selección. Lo mismo pasaba con la "v" al pegar.
 *
 * Aquí cada evento lleva sus modificadores escritos en él mismo
 * (`CGEventSetFlags`), así que no depende del estado del teclado físico.
 * Además, `modifiers_held` permite esperar a que el usuario suelte el atajo
 * antes de mandar nada.
 */

use std::ffi::c_void;
use std::thread::sleep;
use std::time::Duration;

type CGEventRef = *mut c_void;
type CGEventSourceRef = *mut c_void;
type CGKeyCode = u16;
type CGEventFlags = u64;

/// kCGEventSourceStateHIDSystemState: el estado del teclado físico.
const HID_SYSTEM_STATE: i32 = 1;
/// kCGHIDEventTap: el evento entra como si viniera del teclado.
const HID_EVENT_TAP: u32 = 0;

const FLAG_SHIFT: CGEventFlags = 0x0002_0000;
const FLAG_CONTROL: CGEventFlags = 0x0004_0000;
const FLAG_OPTION: CGEventFlags = 0x0008_0000;
const FLAG_COMMAND: CGEventFlags = 0x0010_0000;
const ANY_MODIFIER: CGEventFlags = FLAG_SHIFT | FLAG_CONTROL | FLAG_OPTION | FLAG_COMMAND;

const KEY_COMMAND: CGKeyCode = 0x37;
/// Códigos físicos de las teclas C y V (kVK_ANSI_C / kVK_ANSI_V). Están en el
/// mismo sitio en QWERTY, español, latinoamericano, AZERTY y QWERTZ.
pub const KEY_C: CGKeyCode = 0x08;
pub const KEY_V: CGKeyCode = 0x09;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceCreate(state: i32) -> CGEventSourceRef;
    fn CGEventSourceFlagsState(state: i32) -> CGEventFlags;
    fn CGEventCreateKeyboardEvent(
        source: CGEventSourceRef,
        key: CGKeyCode,
        key_down: bool,
    ) -> CGEventRef;
    fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);
    fn CGEventPost(tap: u32, event: CGEventRef);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

/// ¿Hay alguna tecla modificadora (Cmd, Ctrl, Option, Shift) pulsada ahora
/// mismo en el teclado físico?
pub fn modifiers_held() -> bool {
    let flags = unsafe { CGEventSourceFlagsState(HID_SYSTEM_STATE) };
    flags & ANY_MODIFIER != 0
}

/// Manda Cmd+<tecla> con Cmd marcado en cada evento.
pub fn send_cmd_shortcut(key: CGKeyCode) {
    unsafe {
        let source = CGEventSourceCreate(HID_SYSTEM_STATE);
        post(source, KEY_COMMAND, true, FLAG_COMMAND);
        sleep(Duration::from_millis(10));
        post(source, key, true, FLAG_COMMAND);
        sleep(Duration::from_millis(10));
        post(source, key, false, FLAG_COMMAND);
        sleep(Duration::from_millis(10));
        post(source, KEY_COMMAND, false, 0);
        if !source.is_null() {
            CFRelease(source as *const c_void);
        }
    }
}

unsafe fn post(source: CGEventSourceRef, key: CGKeyCode, key_down: bool, flags: CGEventFlags) {
    // `source` puede ser nulo: CoreGraphics lo acepta y usa uno por defecto.
    let event = CGEventCreateKeyboardEvent(source, key, key_down);
    if event.is_null() {
        return;
    }
    CGEventSetFlags(event, flags);
    CGEventPost(HID_EVENT_TAP, event);
    CFRelease(event as *const c_void);
}
