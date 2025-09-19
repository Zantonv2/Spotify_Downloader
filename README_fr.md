# Téléchargeur Spotify

Une application de bureau qui vous permet de télécharger de la musique depuis Spotify avec tous les métadonnées, pochettes et paroles intacts. Construite avec Rust et React parce que je voulais quelque chose de rapide et fiable.

## Ce que ça fait

En gros, vous recherchez de la musique sur Spotify et la téléchargez sur votre ordinateur. L'application gère toutes les choses techniques - trouver les meilleures sources audio, convertir les formats, intégrer les métadonnées, et s'assurer que tout a l'air bien dans votre lecteur de musique.

## Fonctionnalités

### Les bases
- Rechercher et télécharger depuis Spotify
- Formats audio multiples (MP3, M4A, FLAC, WAV, OGG, Opus, APE)
- Télécharger des playlists ou albums entiers
- Système de file d'attente pour que vous puissiez télécharger un tas de trucs à la fois

### Trucs de métadonnées
- Récupère toutes les infos de la piste (titre, artiste, album, année, etc.)
- Télécharge des pochettes haute résolution (essaie Spotify d'abord, puis iTunes, puis Cover Art Archive)
- Trouve et intègre les paroles
- S'assure que tout est précis après le téléchargement

### Performance
- Télécharge plusieurs pistes en même temps
- Réessaie automatiquement les téléchargements échoués
- Utilise votre GPU pour accélérer le traitement audio
- Met en cache des trucs pour le rendre plus rapide

## Installation

Vous aurez besoin de :
- Rust (le plus récent)
- Node.js (18+)
- Python (3.8+)
- FFmpeg
- yt-dlp

Puis juste :
```bash
git clone https://github.com/Zantonv2/spotify_downloader.git
cd spotify_downloader
cd src-tauri && cargo build
cd ../python_processor && pip install -r requirements.txt
cd .. && npm install
npm run tauri dev
```

## Formats audio

| Format | Qualité | Sans perte ? | Notes |
|--------|---------|--------------|-------|
| MP3 | 128-320 kbps | Non | Le plus compatible |
| M4A | 128-320 kbps | Non | Format Apple |
| FLAC | Sans perte | Oui | Meilleure qualité |
| WAV | Sans perte | Oui | Non compressé |
| OGG | 128-320 kbps | Oui | Open source |
| Opus | 128-320 kbps | Non | Efficace |
| APE | Sans perte | Oui | Haute compression |

## Comment utiliser

1. Recherchez de la musique
2. Cliquez sur "Add to Queue" sur les pistes que vous voulez
3. Allez à l'onglet Downloads et cliquez sur "Download All"
4. Attendez que ça finisse

Vous pouvez aussi importer des playlists entières depuis Spotify si vous avez l'URL de la playlist.

## Paramètres

Il y a un panneau de paramètres où vous pouvez :
- Choisir votre format audio préféré et qualité
- Définir combien de téléchargements exécuter à la fois
- Choisir où sauvegarder les fichiers
- Activer/désactiver l'accélération GPU
- Allumer/éteindre les paroles

## Dépannage

**Les téléchargements ne marchent pas ?**
- Assurez-vous d'avoir FFmpeg installé
- Mettez à jour yt-dlp : `pip install --upgrade yt-dlp`
- Vérifiez votre connexion internet
- Essayez de réduire la limite de téléchargement concurrent

**Pas de pochettes ?**
- Vérifiez votre connexion internet
- L'application essaie plusieurs sources, donc c'est généralement un problème de réseau
- Certaines pistes n'ont simplement pas de pochettes disponibles

**Téléchargements lents ?**
- Activez l'accélération GPU dans les paramètres
- Utilisez un SSD si possible
- Ne mettez pas la limite concurrente trop haute

**Problèmes de qualité audio ?**
- Assurez-vous que FFmpeg est correctement installé
- Essayez un format audio différent
- Vérifiez si l'accélération GPU fonctionne

## Détails techniques

L'application est construite avec :
- **Rust** pour le backend (rapide et fiable)
- **React + TypeScript** pour l'UI
- **Tauri** pour tout assembler
- **yt-dlp** pour trouver et télécharger l'audio
- **FFmpeg** pour traiter l'audio
- **mutagen** (Python) pour gérer les métadonnées

## Pourquoi j'ai construit ça

J'en avais marre des autres téléchargeurs qui soit ne marchaient pas bien, avaient des métadonnées terribles, ou avaient l'air horrible. Donc j'ai construit le mien qui fait tout bien - téléchargements rapides, métadonnées parfaites, belles pochettes, et une interface propre.

## Trucs légaux

C'est pour usage personnel seulement. Ne soyez pas un connard et respectez les lois de copyright. Je ne suis pas responsable si vous utilisez ça pour quelque chose de louche.

## Contribuer

N'hésitez pas à soumettre des issues ou des pull requests. Le code est assez direct - backend Rust, frontend React, Python pour le traitement audio.

## Licence

Licence MIT. Faites-en ce que vous voulez.

---

Si vous trouvez ça utile, considérez mettre une étoile au repo. Si vous trouvez des bugs, ouvrez un issue. Si vous voulez ajouter des fonctionnalités, soumettez un PR.

Bon téléchargement ! 🎵
