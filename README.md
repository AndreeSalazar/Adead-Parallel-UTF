# adead-parallel-utf

> **SSD is the source of truth. RAM is a temporary shadow.**

## 🚀 Problema que resuelve
En sistemas de alto rendimiento (Juegos AAA, Motores, IA), cargar terabytes de texto en RAM es inviable y serializar/deserializar es lento. `adead-parallel-utf` elimina la necesidad de cargar strings en RAM.

## 💡 Solución: UTF-Paralelo
- **Persistencia Inmutable**: Los datos viven en NVMe/SSD.
- **RAM Efímera**: La RAM solo guarda IDs y Offsets. Si falta memoria, se olvida, pero los datos persisten.
- **Zero-Copy**: Usamos `mmap` para proyectar el disco en memoria virtual sin copias costosas.

## 🏗 Arquitectura
1. **Store**: Append-only log en SSD.
2. **Index**: `DashMap` en RAM (ID -> Offset).
3. **Resolver**: Lazy loading vía page-faults del SO.

## 🛠 Stack
- **Rust**: Control absoluto de memoria y threads.
- **mmap**: Acceso directo al kernel.
- **rayon**: Paralelismo de datos masivo.

## 📦 Uso

```rust
use adead_parallel_utf::Resolver;

fn main() -> anyhow::Result<()> {
    let resolver = Resolver::new("data.puf")?;
    let id = resolver.register_utf("Texto masivo...")?; // Escribe en SSD
    
    if let Some(text_ref) = resolver.resolve_utf(id) {
        println!("Texto: {}", &*text_ref); // Lee de mmap (Zero-copy)
    }
    Ok(())
}
```

## 🏆 Casos de Uso
- ✔ Juegos AAA (Texturas de texto, scripts)
- ✔ Motores Gráficos
- ✔ IA Local (RAG, Context retrieval)
- ✔ Edge Computing

---
*Construido para la era del NVMe.*
