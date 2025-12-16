# 🧠 Ideas & Arquitectura: SSD-First, RAM-Shadow

Este documento detalla la filosofía, mecanismos y hoja de ruta para lograr que la RAM sea una "sombra" eficiente mientras el SSD asume la carga pesada.

## 🎯 Objetivo Central
**Invertir la jerarquía de memoria tradicional.**
En lugar de "Cargar todo a RAM para que sea rápido", el modelo es "Dejar todo en SSD y mapear solo lo necesario".

> **Meta:** Lograr un throughput de lectura de texto cercano a la velocidad nativa del NVMe (3-7 GB/s) con un uso de RAM constante y predecible (O(1) o O(N_indices)), independiente del tamaño total de los datos.

## 1️⃣ Principios de Cooperación RAM-SSD

### La RAM como "Mapa", no como "Territorio"
- **Rol de la RAM**: Solo debe conocer la *existencia* y *ubicación* de los datos.
  - Estructura: `Hash(ID) -> { Offset, Length }`.
  - Costo: ~16 bytes por entrada. 1 millón de textos = ~16 MB de RAM.
- **Rol del SSD**: Contiene el *cuerpo* de los datos.
  - Estructura: `[Header][Entry][Data][Entry][Data]...`
  - Inmutable: Una vez escrito, no se mueve. Esto permite que el SO cachee agresivamente sin problemas de coherencia.

### El Sistema Operativo es el Cache Manager
- No reinventar la rueda. Linux/Windows gestionan la memoria virtual mejor que nosotros.
- **Mecanismo**: `mmap` (Memory Mapped File).
  - Cuando accedemos a un byte, si no está en RAM, el CPU dispara un *Page Fault*.
  - El SO pausa el hilo (microsegundos), carga la página de 4KB desde el NVMe a la RAM, y reanuda.
  - Si la RAM se llena, el SO descarta las páginas más viejas (LRU nativo del kernel).
  - **Ventaja**: No necesitamos un Garbage Collector ni un Cache Manager complejo en userspace.

## 2️⃣ Estrategias de Optimización (Cómo compensar la latencia)

Aunque el NVMe es rápido, es más lento que la RAM. Para compensar:

### A. Paralelismo Masivo (Rayon)
- Los NVMe modernos tienen múltiples colas de hardware (NVMe Queues).
- Leer 1 string es lento (latencia). Leer 10,000 strings en paralelo satura el ancho de banda.
- **Implementación**: Usar `rayon` para disparar múltiples *page faults* en paralelo. El controlador del SSD reordenará las peticiones para máxima eficiencia.

### B. Prefetching Inteligente (Coop. Activa)
- **El problema**: El SO es reactivo (espera al Page Fault).
- **La solución**: Ser proactivos.
  - `madvise(MADV_WILLNEED)`: Decirle al kernel "Voy a usar este rango de memoria pronto".
  - El kernel inicia la lectura DMA asíncrona desde el NVMe a la RAM *antes* de que el código llegue ahí.
  - **Resultado**: Cuando el CPU pide el dato, ya está en RAM. Latencia cercana a cero.

### C. Estructura de Datos "Friendly" para Hardware
- **Alineación**: Asegurar que los datos comiencen en múltiplos de página (4KB) para textos muy grandes (opcional, pero útil para Zero-Copy real).
- **Contigüidad**: Textos relacionados deberían escribirse juntos en el archivo `PUF` para aprovechar la localidad espacial.

## 3️⃣ Metas Técnicas para "Ganar" a la RAM tradicional

1.  **Tiempo de Inicio Instantáneo**:
    - RAM tradicional: Tiene que leer y parsear todo el archivo al inicio (lento).
    - ADead-Parallel: Solo lee el índice (o incluso mapea el índice). Inicio en milisegundos.

2.  **Resiliencia a Crashes**:
    - Si el proceso muere, los datos ya están en disco. No hay `fsync` de pánico necesario.

3.  **Escalabilidad Infinita**:
    - Puedes tener un dataset de 10 TB en una máquina con 16 GB de RAM.
    - El rendimiento se degrada suavemente (thrashing) en lugar de crashear por OOM (Out of Memory).

## 4️⃣ Inteligencia Cooperativa Avanzada (NUEVO)

Para llevar la cooperación al siguiente nivel, implementaremos:

### 🧠 A. Predicción de Acceso (Heurística)
- Si el usuario accede a `ID_100`, es probable que acceda a `ID_101` (localidad temporal).
- El sistema puede disparar un *prefetch* especulativo de los vecinos en el archivo físico.

### 🧊 B. Hot/Cold Tiering (Optimización de Layout)
- **Problema**: Con el tiempo, los datos "calientes" (muy usados) quedan dispersos entre datos "fríos" (viejos).
- **Solución Inteligente**:
  - Un proceso background analiza estadísticas de acceso.
  - Reescribe un nuevo archivo `.puf` colocando todos los datos "Hot" juntos al principio.
  - **Beneficio**: Maximiza el uso de cada página de 4KB en RAM (densidad de información).

### 📦 C. Compresión Híbrida
- Textos pequeños (< 64 bytes): Guardar raw (la descompresión es más cara que la lectura).
- Textos grandes (> 4KB): Comprimir con LZ4/Zstd.
  - NVMe lee menos bytes -> Menos presión en bus PCIe.
  - CPU descomprime rápido en L3 cache.

### ⚡ D. Async I/O Profundo (io_uring / IOCP)
- Para cargas extremas, saltar el `mmap` y usar I/O asíncrono directo (`O_DIRECT`) para llenar buffers de usuario, evitando la gestión de páginas del SO si detectamos que el patrón de acceso es completamente aleatorio y masivo.

---

### 📝 Resumen Ejecutivo
La "ventaja injusta" de esta arquitectura es que **delega la complejidad al hardware y al kernel**. Mientras otros pelean gestionando buffers en heap, nosotros dejamos que el MMU (Memory Management Unit) y el controlador NVMe hagan lo que mejor saben hacer: mover bits rapidísimo.
