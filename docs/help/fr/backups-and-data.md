# Sauvegardes et données

Vos notes vivent sur votre PC, dans un seul dossier, et Dobo les sauvegarde chaque jour. Pas de compte, rien à quoi se connecter.

## Où sont vos notes

**Paramètres › À propos** indique quel Dobo vous avez et où sont ses notes :

- **Installée** (avec Dobo-Setup.exe) : « Installée sur cet ordinateur. Mettre à jour ou réinstaller garde vos notes. » Les notes sont dans votre dossier utilisateur Windows, `%APPDATA%\com.sookera.ztodo`.
- **Portable** (Dobo.exe tout seul) : « Portable : notes, images et sauvegardes restent dans le dossier data à côté de Dobo.exe. Déplacez ou copiez le dossier entier. » Un Dobo portable utilise plutôt `%APPDATA%\com.sookera.ztodo` quand son propre dossier n’accepte pas l’écriture (dans Program Files, sur un disque en lecture seule), ou quand des notes d’un Dobo installé s’y trouvent déjà et que son dossier `data` n’en a pas encore. Voir [Installer et mettre à jour](install-and-update.md#dobo-portable).

**Dossier des données** montre le dossier lui-même ; **Ouvrir le dossier** l’ouvre dans l’Explorateur. On y trouve vos notes, les images ajoutées, les images des cartes de liens, votre image de fond et, sauf si vous avez choisi un autre dossier, vos sauvegardes.

## Les sauvegardes quotidiennes

**Paramètres › Sauvegardes** :

- **Sauvegarder chaque jour** (activé au départ) : une copie de vos notes et images, une fois par jour. Les 14 dernières sont gardées.
- **Dossier** : où vont les sauvegardes. Au départ, un dossier `backups` dans le dossier des données. **Choisir…** en prend un autre (un disque synchronisé ou externe, par exemple) ; **Par défaut** y revient. Désinstaller Dobo ne supprime pas un dossier que vous avez choisi : il est à vous, supprimez-le vous-même si vous le souhaitez.
- **Sauvegarder** en fait une tout de suite.
- **Vos sauvegardes** les liste, de la plus récente à la plus ancienne, avec leur raison (**Quotidienne**, **À la main**, **Avant une restauration**) et leur taille.

Un dossier de sauvegardes hors du dossier des données est plus sûr : il reste même si le dossier des données est supprimé.

## Restaurer une sauvegarde

Dans **Vos sauvegardes**, cliquez sur **Restaurer…** à côté de l’une d’elles, puis **Restaurer celle-ci** (ou **Garder les miennes** si vous changez d’avis). Avant de restaurer, Dobo sauvegarde vos notes actuelles, rien ne se perd : « Restauré. Vos notes d’avant sont gardées en sauvegarde, elles aussi. »

## Emporter vos notes sur un autre PC

1. Sur l’ancien PC, cliquez sur **Sauvegarder**, puis copiez le dossier des sauvegardes (ou seulement la plus récente) sur une clé USB ou un disque partagé.
2. Sur le nouveau PC, installez Dobo, ouvrez **Paramètres › Sauvegardes › Dossier › Choisir…** et prenez le dossier copié.
3. Cliquez sur **Restaurer…** à côté de la sauvegarde, puis **Restaurer celle-ci**. Vos notes et vos images reviennent telles quelles.

Avec un Dobo portable, c’est plus simple : copiez tout son dossier, `Dobo.exe` et `data` ensemble.

Votre style (thèmes, styles, mélanges, couleurs) peut aussi voyager seul, dans un fichier `.ztheme` : voir [Style et thèmes](style-and-themes.md#partager-un-style-ou-lemporter-sur-un-autre-pc).

## Pense-bêtes de Windows

Au tout premier lancement, tant qu’il n’y a encore aucune note, Dobo reprend de lui-même vos notes de Pense-bêtes, sans demander. Il lit la base de données de Pense-bêtes sur ce PC, depuis une copie : vos Pense-bêtes ne sont jamais modifiés, et rien ne quitte votre PC. Reprenez-les de nouveau à tout moment depuis **Paramètres › Démarrage › Reprendre les notes de Pense-bêtes (Windows)** (**Importer**). Les notes déjà sur le tableau ne sont pas ajoutées deux fois.
