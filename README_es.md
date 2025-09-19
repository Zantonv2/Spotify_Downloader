# Descargador de Spotify

Una aplicación de escritorio que te permite descargar música de Spotify con todos los metadatos, portadas y letras intactos. Construida con Rust y React porque quería algo rápido y confiable.

## Lo que hace

Básicamente, buscas música en Spotify y la descargas a tu computadora. La aplicación maneja todas las cosas técnicas - encontrar las mejores fuentes de audio, convertir formatos, incrustar metadatos, y asegurarse de que todo se vea bien en tu reproductor de música.

## Características

### Lo básico
- Buscar y descargar desde Spotify
- Múltiples formatos de audio (MP3, M4A, FLAC, WAV, OGG, Opus, APE)
- Descargar listas de reproducción o álbumes completos
- Sistema de cola para que puedas descargar un montón de cosas a la vez

### Cosas de metadatos
- Obtiene toda la información de la pista (título, artista, álbum, año, etc.)
- Descarga portadas de alta resolución (intenta Spotify primero, luego iTunes, luego Cover Art Archive)
- Encuentra e incrusta letras
- Se asegura de que todo sea preciso después de la descarga

### Rendimiento
- Descarga múltiples pistas al mismo tiempo
- Reintenta descargas fallidas automáticamente
- Usa tu GPU para acelerar el procesamiento de audio
- Cachea cosas para hacerlo más rápido

## Instalación

Necesitarás:
- Rust (más reciente)
- Node.js (18+)
- Python (3.8+)
- FFmpeg
- yt-dlp

Entonces solo:
```bash
git clone https://github.com/Zantonv2/spotify_downloader.git
cd spotify_downloader
cd src-tauri && cargo build
cd ../python_processor && pip install -r requirements.txt
cd .. && npm install
npm run tauri dev
```

## Formatos de audio

| Formato | Calidad | ¿Sin pérdida? | Notas |
|---------|---------|---------------|-------|
| MP3 | 128-320 kbps | No | Más compatible |
| M4A | 128-320 kbps | No | Formato Apple |
| FLAC | Sin pérdida | Sí | Mejor calidad |
| WAV | Sin pérdida | Sí | Sin comprimir |
| OGG | 128-320 kbps | Sí | Código abierto |
| Opus | 128-320 kbps | No | Eficiente |
| APE | Sin pérdida | Sí | Alta compresión |

## Cómo usar

1. Busca música
2. Haz clic en "Add to Queue" en las pistas que quieras
3. Ve a la pestaña Downloads y haz clic en "Download All"
4. Espera a que termine

También puedes importar listas de reproducción completas desde Spotify si tienes la URL de la lista de reproducción.

## Configuración

Hay un panel de configuración donde puedes:
- Elegir tu formato de audio preferido y calidad
- Establecer cuántas descargas ejecutar a la vez
- Elegir dónde guardar archivos
- Habilitar/deshabilitar aceleración GPU
- Activar/desactivar letras

## Solución de problemas

**¿Las descargas no funcionan?**
- Asegúrate de tener FFmpeg instalado
- Actualiza yt-dlp: `pip install --upgrade yt-dlp`
- Verifica tu conexión a internet
- Intenta reducir el límite de descarga concurrente

**¿Sin portadas?**
- Verifica tu conexión a internet
- La aplicación intenta múltiples fuentes, así que esto es usualmente un problema de red
- Algunas pistas simplemente no tienen portadas disponibles

**¿Descargas lentas?**
- Habilita la aceleración GPU en configuración
- Usa un SSD si es posible
- No establezcas el límite concurrente muy alto

**¿Problemas de calidad de audio?**
- Asegúrate de que FFmpeg esté instalado correctamente
- Prueba un formato de audio diferente
- Verifica si la aceleración GPU está funcionando

## Detalles técnicos

La aplicación está construida con:
- **Rust** para el backend (rápido y confiable)
- **React + TypeScript** para la UI
- **Tauri** para juntarlo todo
- **yt-dlp** para encontrar y descargar audio
- **FFmpeg** para procesar el audio
- **mutagen** (Python) para manejar metadatos

## Por qué construí esto

Me cansé de otros descargadores que o no funcionaban bien, tenían metadatos terribles, o se veían horribles. Así que construí el mío que hace todo bien - descargas rápidas, metadatos perfectos, portadas hermosas, y una interfaz limpia.

## Cosas legales

Esto es solo para uso personal. No seas un idiota y respeta las leyes de derechos de autor. No soy responsable si usas esto para algo sospechoso.

## Contribuir

Siéntete libre de enviar issues o pull requests. El código es bastante directo - backend Rust, frontend React, Python para procesamiento de audio.

## Licencia

Licencia MIT. Haz lo que quieras con ella.

---

Si encuentras esto útil, considera darle una estrella al repositorio. Si encuentras bugs, abre un issue. Si quieres agregar características, envía un PR.

¡Feliz descarga! 🎵
