---
name: gan-gen4-protocol
description: Especificación técnica del protocolo BLE GAN Gen4 y representación del cubo 3x3.
---
- El cubo GAN i4 usa encriptación AES-128-CBC/ECB cuya llave e IV se derivan de los 6 bytes de la dirección MAC del dispositivo.
- Cada paquete decodificado debe validarse por su cabecera de evento: movimiento de capa (cara + dirección + timestamp hardware en ms), estado completo de 54 stickers/piezas (anti-desync) o giroscopio (cuaternión `w, x, y, z`).
- El reloj maestro de cada solve es el timestamp interno en milisegundos reportado por el cubo, nunca el momento de recepción del paquete BLE.