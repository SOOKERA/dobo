# Sauvegardes et données

Vos notes vivent sur votre PC, dans un seul dossier, et Dobo les sauvegarde chaque jour. Pas de compte, rien à quoi se connecter.

## Où sont vos notes

**Paramètres › À propos** indique quel Dobo vous avez et où sont ses notes :

- **Installée** (avec Dobo-Setup.exe) : « Installée sur cet ordinateur. Mettre à jour ou réinstaller garde vos notes. » Les notes sont dans votre dossier utilisateur Windows, `%APPDATA%\com.sookera.ztodo`.
- **Portable** (Dobo.exe tout seul) : « Portable : notes, images et sauvegardes restent dans le dossier data à côté de Dobo.exe. Déplacez ou copiez le dossier entier. » Un Dobo portable utilise plutôt `%APPDATA%\com.sookera.ztodo` quand des notes d’un Dobo installé s’y trouvent déjà et que son dossier `data` n’en a pas encore. Quand son propre dossier n’accepte pas l’écriture (dans Program Files, sur un disque en lecture seule), il demande d’abord : voir [Installer et mettre à jour](install-and-update.md#dobo-portable).

**Dossier des données** montre le dossier lui-même ; **Ouvrir le dossier** l’ouvre dans l’Explorateur. On y trouve vos notes, les images ajoutées, les images des cartes de liens, votre image de fond et, sauf si vous avez choisi un autre dossier, vos sauvegardes. Le dossier `pictures\thumbs` contient des copies plus petites de vos images, que le tableau affiche pour s’ouvrir vite ; Dobo les refait chaque fois qu’elles manquent.

## Les sauvegardes quotidiennes

**Paramètres › Sauvegardes** :

- **Sauvegarder chaque jour** (activé au départ) : une copie de vos notes et images, une fois par jour. Les 14 dernières sont gardées. (Les copies plus petites de `pictures\thumbs` ne sont pas sauvegardées : après une restauration, Dobo les refait, quelques-unes à la fois.)
- **Dossier** : où vont les sauvegardes. Au départ, un dossier `backups` dans le dossier des données, et une ligne discrète le rappelle dessous (comme pour tout dossier sur le même disque ou partage réseau que vos notes) : « Gardées avec vos notes, sur le même disque. Un dossier sur un autre disque les protège si celui-ci lâche. » **Choisir…** en prend un autre (un disque synchronisé ou externe, par exemple) ; **Par défaut** y revient. Désinstaller Dobo ne supprime pas un dossier que vous avez choisi : il est à vous, supprimez-le vous-même si vous le souhaitez.
- **Sauvegarder** en fait une tout de suite.
- **Vos sauvegardes** les liste par type, de la plus récente à la plus ancienne, chacune avec sa date et sa taille. Une sauvegarde faite avant une mise à jour qui n’a pas pu aller dans le dossier que vous avez choisi (voir [Installer et mettre à jour](install-and-update.md)) y est aussi, « dans le dossier des données ».

Chaque type garde ses propres dernières sauvegardes : en faire à la main ne chasse jamais les quotidiennes.

| Sorte | Faite | Gardées |
|---|---|---|
| **Quotidienne** | une fois par jour | les 14 dernières |
| **À la main** | avec **Sauvegarder** | les 10 dernières |
| **Avant une restauration** | par **Restaurer…**, des notes que vous aviez | les 5 dernières |
| **Avant une mise à jour** | quand une nouvelle version de Dobo ouvre vos notes pour la première fois, avant d’y ajouter quoi que ce soit (« Pour Dobo 1.1.0 », avec le numéro de la version) | les 3 dernières |

Un dossier de sauvegardes hors du dossier des données est plus sûr : il reste même si le dossier des données est supprimé, ou si le disque lâche.

## Restaurer une sauvegarde

Dans **Vos sauvegardes**, cliquez sur **Restaurer…** à côté de l’une d’elles, puis **Restaurer celle-ci** (ou **Garder les miennes** si vous changez d’avis). Avant de restaurer, Dobo sauvegarde vos notes actuelles, rien ne se perd : « Restauré. Vos notes d’avant sont gardées en sauvegarde, elles aussi. »

La sauvegarde ramène ses notes, ses images et ses paramètres, pas ses journées : un bilan du soir déjà fermé reste fermé, et **Faire le tri** ne sonne pas une nouvelle fois aujourd’hui. Elle ne ramène pas non plus les ordinateurs depuis lesquels vous avez autorisé l’ouverture de fichiers (voir [Tâches et idées](tasks-and-ideas.md#images-et-fichiers)), ni son dossier des sauvegardes (une sauvegarde venue d’un autre PC garde celui de ce PC), ni ce qu’elle disait des sauvegardes à son époque.

Si vous écrivez pendant une restauration, ce que vous écrivez va aussi dans la sauvegarde « Avant une restauration ». Si le disque de cette sauvegarde a disparu juste à ce moment, Paramètres › Sauvegardes indique où ces changements sont gardés.

## Si vos notes ne s’ouvrent pas

Rarement, le fichier des notes ne peut pas être ouvert au démarrage de Dobo : une erreur de disque l’a endommagé, un autre programme le tient, ou il ne peut pas être modifié. Dobo affiche alors un écran calme qui dit ce qui se passe, au lieu de ne pas démarrer :

- **Le fichier de vos notes est endommagé.** Rien n’est renommé tant que vous n’avez pas choisi. Dobo essaie d’abord de le réparer : quand tout ce qu’il peut encore lire tient dans un nouveau fichier, **Utiliser les notes réparées** vient en premier, avec le nombre de notes et de lignes qu’elle contient (une réparation peut en manquer, comparez donc avec la sauvegarde). Ensuite, **Réessayer** relit le fichier (un lecteur qui a mal répondu une fois peut le lire en entier maintenant), **Restaurer la sauvegarde la plus récente** (sa date et son nombre de notes et de lignes sont affichés : la plus récente qui est intacte), ou **Commencer avec un tableau vide** (vos paramètres suivent s’ils peuvent encore être lus). Chacun de ces choix met d’abord le fichier endommagé de côté, tel quel, sous le nom `broken-<date>.sqlite` dans le dossier des données : rien n’est supprimé, et les images qu’il nomme sont gardées tant qu’il est là. Si le choix ne peut pas être fait, le fichier reprend sa place. **Quitter Dobo** laisse tout tel quel, et l’écran revient au démarrage suivant.
- **Vos notes peuvent être récupérées** : un démarrage précédent a mis vos notes de côté parce qu’elles ne pouvaient pas être lues, et elles se lisent en entier maintenant. **Récupérer mes notes** vous fait reprendre là où vous en étiez.
- **Un autre programme utilise vos notes** (souvent un antivirus, un outil de synchronisation ou de sauvegarde) : rien n’est renommé. Patientez un instant, puis **Réessayer**.
- **Vos notes ne peuvent pas être enregistrées** : Windows laisse Dobo lire le fichier mais pas le modifier (le fichier ou son dossier est en lecture seule). Décochez **Lecture seule** dans les Propriétés du fichier, puis **Réessayer**.
- **Vos notes ne se sont pas ouvertes**, pour une autre raison : **Réessayer** d’abord ; ensuite, vous pouvez restaurer la sauvegarde la plus récente ou commencer avec un tableau vide, et le fichier est mis de côté dans les deux cas (et reprend sa place si ce n’est pas possible).
- **Le tableau n’a pas pu démarrer** : votre choix a marché, mais le tableau ne s’est pas affiché. **Réessayer** ; s’il ne démarre toujours pas, quittez Dobo et rouvrez-le. Vos notes restent telles quelles.

**Ouvrir le dossier des données** montre le dossier dans l’Explorateur. **Haut** / **Bas** passent d’un choix à l’autre, **Entrée** le choisit.

## Emporter vos notes sur un autre PC

1. Sur l’ancien PC, cliquez sur **Sauvegarder**, puis copiez le dossier des sauvegardes (ou seulement la plus récente) sur une clé USB ou un disque partagé.
2. Sur le nouveau PC, installez Dobo, ouvrez **Paramètres › Sauvegardes › Dossier › Choisir…** et prenez le dossier copié.
3. Cliquez sur **Restaurer…** à côté de la sauvegarde, puis **Restaurer celle-ci**. Vos notes et vos images reviennent telles quelles.

Avec un Dobo portable, c’est plus simple : copiez tout son dossier, `Dobo.exe` et `data` ensemble.

Votre style (thèmes, styles, mélanges, couleurs) peut aussi voyager seul, dans un fichier `.ztheme` : voir [Style et thèmes](style-and-themes.md#partager-un-style-ou-lemporter-sur-un-autre-pc).

## Pense-bêtes de Windows

Au premier lancement, si Pense-bêtes contient des notes, la fenêtre d’accueil demande : « Récupérer vos 4 notes de Pense-bêtes ? » Rien n’arrive tant que vous ne cliquez pas sur **Importer**. **Pas maintenant** les laisse où elles sont, et Dobo ne redemande pas. Dans les deux cas, vous pouvez les récupérer à tout moment depuis **Paramètres › Démarrage › Récupérer les notes de Pense-bêtes (Windows)** (**Importer**). Dobo lit la base de données de Pense-bêtes sur ce PC, depuis une copie : vos Pense-bêtes ne sont jamais modifiés, et rien ne quitte votre PC. Les notes déjà sur le tableau ne sont pas ajoutées deux fois.
