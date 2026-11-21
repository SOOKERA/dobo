# Installer et mettre à jour

Obtenir Dobo, le garder à jour, et le retirer si besoin. Dobo fonctionne sous Windows 10 et 11.

## Dobo-Setup ou Dobo portable

Dobo existe en deux versions, sur [dobo.sookera.com](https://dobo.sookera.com). C’est le même programme, et les deux se mettent à jour de la même façon.

- **Dobo-Setup.exe** installe Dobo pour vous seul (par utilisateur), sans droits d’administrateur, dans `%LOCALAPPDATA%\Programs\Dobo`. Il ajoute Dobo au menu Démarrer (et au bureau si vous voulez) et à la liste des applications de Windows, pour le désinstaller comme n’importe quelle appli.
- **Dobo.exe** est la version portable : un seul fichier. Mettez-le dans un dossier à lui et lancez-le. Il n’est ajouté ni au menu Démarrer ni à la liste des applications.

**Paramètres › À propos** indique laquelle vous avez, **Installée** ou **Portable**, à côté de la version, et où sont ses notes.

### La fenêtre de Dobo Setup

La fenêtre de Dobo Setup est en anglais :

- **Install in** : le dossier, `%LOCALAPPDATA%\Programs\Dobo` au départ. **Change** en choisit un autre. Ce doit être un dossier que vous pouvez modifier sans droits d’administrateur (pas Program Files), car Dobo s’y met à jour lui-même : sinon, Setup dit pourquoi et bloque le bouton jusqu’à ce que vous en choisissiez un autre.
- **Desktop shortcut** : un raccourci sur le bureau (activé au départ).
- **Start Dobo with Windows** : lancer Dobo avec Windows (activé au départ). Les rappels ne sonnent que si Dobo tourne.
- Cliquez sur **Install**. Installer vaut acceptation des conditions d’utilisation de Dobo (« By installing, you accept the terms of use. »), en lien sous le bouton : le Dobo installé ne les redemande donc pas. À la fin, **Open Dobo** le lance.

Si Dobo est ouvert, Setup le ferme d’abord : vos notes sont enregistrées au fil de l’écriture, rien n’est perdu.

### Réinstaller après une désinstallation

Setup se souvient du dossier où Dobo était installé. Si vous avez désinstallé Dobo en gardant vos notes, une nouvelle installation propose ce même dossier sous **Install in** (vous pouvez toujours le changer avec **Change**), et vos notes sont là, comme vous les avez laissées. Si vous avez désinstallé avec **Also delete my notes** activé, ce dossier est oublié lui aussi, et Setup revient à `%LOCALAPPDATA%\Programs\Dobo`.

### Dobo portable

Au premier lancement, Dobo.exe crée un dossier `data` à côté de lui et y garde tout : notes, images, sauvegardes, et les données de sa fenêtre. Déplacez ou copiez le dossier entier (`Dobo.exe` et `data` ensemble) pour l’emporter, sur une clé USB par exemple.

Deux cas où il utilise plutôt le dossier habituel de votre profil (`%APPDATA%\com.sookera.ztodo`) :

- **Ce PC a déjà des notes Dobo à cet endroit** (d’un Dobo installé, ou d’une copie précédente) et le dossier `data` n’en a pas encore : le Dobo portable continue de les utiliser, pour que vos notes ne soient jamais laissées derrière ni coupées en deux. **Paramètres › À propos** dit « Portable, avec les notes déjà présentes sur cet ordinateur. » Un dossier `data` qui contient déjà des notes est toujours celui utilisé.
- **Le dossier de Dobo.exe n’accepte pas l’écriture** (dans Program Files, ou sur un disque en lecture seule) : « Portable, mais le dossier de Dobo.exe n’accepte pas l’écriture : les notes sont dans votre profil utilisateur. » Dobo ne peut pas non plus s’y mettre à jour : déplacez le dossier de Dobo là où vous pouvez modifier des fichiers, comme Documents.

Toute personne qui peut ouvrir le dossier portable peut lire les notes qu’il contient : gardez-le dans un dossier à vous.

### Les conditions d’utilisation, au premier lancement

Le Dobo portable affiche ses conditions d’utilisation à son premier lancement : « Avant de commencer, lisez les conditions d’utilisation de Dobo. Dobo démarre dès que vous les acceptez. » Cliquez sur **Accepter et continuer**, ou sur **Quitter Dobo**. Il ne le demande qu’une fois, jamais plus ensuite, même après une mise à jour. Tant que vous n’avez pas accepté, Dobo ne va pas du tout en ligne.

Un Dobo installé ne les demande pas, puisque vous les avez acceptées dans la fenêtre de Setup, sauf après une installation silencieuse (voir plus bas), que personne n’a vue. Si vous utilisiez Dobo avant qu’il ait des conditions, il les affiche une fois aussi (« Dobo a maintenant des conditions d’utilisation »), et vos notes restent telles quelles.

## « Windows a protégé votre ordinateur »

La première fois que vous lancez Dobo-Setup.exe ou Dobo.exe, Windows peut dire qu’il a protégé votre ordinateur. Cliquez sur **Informations complémentaires**, puis **Exécuter quand même**.

La fenêtre de Dobo utilise Microsoft Edge WebView2, déjà présent sur Windows 11 et la plupart des PC sous Windows 10.

## Mises à jour

Ouvrez **Paramètres › Mises à jour** :

- **Chercher les mises à jour automatiquement** (activé au départ) : « Une fois par jour. Une nouvelle version est téléchargée et vérifiée, puis utilisée au prochain démarrage de Dobo. Vos notes ne sont jamais touchées. » Dobo cherche aussi s’il reste ouvert des jours dans la zone de notification. S’il n’arrive pas à joindre internet, il réessaie une heure plus tard.
- **Rechercher les mises à jour** cherche tout de suite, même avec la recherche automatique désactivée.
- La ligne à côté du bouton dit où en sont les choses. Dessous, la ligne de version : « Version 1.0.0 · dernière recherche : … » (ou « pas encore de recherche »). Quand la ligne à côté du bouton ne donne que la version, celle du dessous dit seulement « Dernière recherche : … » ou « Pas encore de recherche ». Quand une nouvelle version est téléchargée et vérifiée, elle dit **Dobo {v} est prêt** (« Cette version démarre à la prochaine ouverture de Dobo, ou tout de suite avec Redémarrer maintenant. »), et **Redémarrer maintenant** remplace le bouton. Le tableau le dit aussi une fois, avec **Redémarrer maintenant** : « Dobo {v} est prêt. Il démarre à la prochaine ouverture de Dobo. »

Comment se passe une mise à jour :

- Dobo télécharge la nouvelle version et la vérifie avant tout : elle doit être signée par Sookera, avec la bonne taille et la bonne somme de contrôle. Tout le reste est refusé, et les Paramètres disent pourquoi.
- Elle est utilisée au prochain démarrage de Dobo, ou tout de suite avec **Redémarrer maintenant**.
- La nouvelle version démarre d’abord à l’essai. Si elle ne démarre pas correctement, le démarrage suivant remet la version que vous aviez, qui continue de marcher, et les Paramètres le disent (« Dobo 1.0.1 n’a pas démarré correctement : Dobo 1.0.0 a été remis. »). La recherche automatique ne retélécharge pas cette version ; **Rechercher les mises à jour** le peut.
- Seul Dobo.exe change. Vos notes, vos réglages et vos styles ne sont jamais touchés.

Avec la recherche automatique désactivée, Dobo ne va jamais en ligne pour les mises à jour, sauf si vous cliquez sur **Rechercher les mises à jour**.

Une version de développement ou de test de Dobo (construite à partir de son code source, pas téléchargée sur dobo.sookera.com) ne se met jamais à jour : **Rechercher les mises à jour** dit « Cette copie de Dobo est une version de développement ou de test : elle ne se met jamais à jour. »

### Un Dobo-Setup plus récent sur un Dobo installé

Vous pouvez aussi lancer un Dobo-Setup.exe plus récent sur le Dobo installé : sa fenêtre dit « Update Dobo », et **Update** le met à jour dans son dossier en gardant vos notes, vos réglages, vos raccourcis et l’épingle de la barre des tâches.

## Installer sans fenêtre

Pour installer sur plusieurs PC, ou depuis un script :

- `Dobo-Setup.exe /S` installe en silence, dans `%LOCALAPPDATA%\Programs\Dobo` (ou dans le dossier où Dobo était installé avant), avec un raccourci sur le bureau.
- `Dobo-Setup.exe /S /D=C:\Outils\Dobo` installe dans un autre dossier. `/D=` vient en dernier, sans guillemets, même si le chemin a des espaces.
- Le dossier doit être modifiable par cet utilisateur Windows. Sinon, Setup s’arrête avant de rien changer, avec le code de sortie 2 et la raison dans la console.
- Une installation silencieuse n’accepte pas les conditions à votre place : Dobo les affiche à son premier lancement. Elle ne règle pas non plus **Start Dobo with Windows** : sur un PC sans notes Dobo, la mise en route de Dobo au premier lancement le propose ; sinon, activez-le dans **Paramètres › Démarrage**.
- `uninstall.exe /S`, dans le dossier de Dobo, désinstalle en silence. Il garde toujours vos notes : **Also delete my notes** ne s’applique jamais à une désinstallation silencieuse.

## Désinstaller

### Désinstaller le Dobo installé

La fenêtre de désinstallation est en anglais, elle aussi.

1. Ouvrez **Paramètres › Applications › Applications installées** de Windows, trouvez Dobo et cliquez sur **Désinstaller**.
2. Activez **Also delete my notes** (supprimer aussi mes notes) seulement si vous voulez aussi les effacer. Dessous : « Your notes, pictures, settings and the backups in Dobo's own folder. A backups folder you chose is kept. This can't be undone. » (vos notes, images, réglages et les sauvegardes du dossier de Dobo ; un dossier de sauvegardes que vous avez choisi est gardé ; on ne peut pas revenir en arrière). C’est désactivé au départ, pour qu’une réinstallation reprenne où vous en étiez. Si vous pourriez vouloir vos notes un jour, gardez d’abord une sauvegarde hors du dossier des données (voir [Sauvegardes et données](backups-and-data.md)). Un dossier de sauvegardes choisi dans **Paramètres › Sauvegardes** n’est jamais supprimé par la désinstallation : supprimez-le vous-même si vous le souhaitez.
3. Confirmez. Dobo, ses raccourcis et son lancement avec Windows sont retirés : « Dobo is gone from this PC. »

Si un autre Dobo est ouvert (une copie portable), vos notes sont gardées même avec **Also delete my notes** activé, car il peut utiliser le même dossier. Fermez-le, puis supprimez vous-même le dossier `com.sookera.ztodo` de `%APPDATA%` si vous voulez qu’elles disparaissent.

### Retirer le Dobo portable

1. Dans Dobo, désactivez **Paramètres › Démarrage › Lancer Dobo avec l’ordinateur**.
2. Quittez Dobo (clic droit sur l’icône › **Quitter Dobo**).
3. Supprimez Dobo.exe et son dossier `data`. Si **Paramètres › À propos** indiquait que ses notes étaient dans votre profil utilisateur, elles sont dans `%APPDATA%\com.sookera.ztodo` (partagé avec un Dobo installé, si vous en avez un). Un dossier de sauvegardes que vous avez choisi reste tant que vous ne le supprimez pas.

## Du portable à l’installé

Le Dobo installé garde ses notes dans votre dossier utilisateur, pas à côté du portable. Pour emporter vos notes :

1. Dans le Dobo portable, ouvrez **Paramètres › Sauvegardes** et cliquez sur **Sauvegarder**.
2. Quittez-le, puis installez Dobo avec Dobo-Setup.exe, dans un autre dossier que celui du portable (Setup en demande un autre, pour que les notes du portable restent avec lui).
3. Dans le Dobo installé, ouvrez **Paramètres › Sauvegardes › Dossier › Choisir…** et prenez le dossier `data\backups` du portable.
4. Cliquez sur **Restaurer…** à côté de la sauvegarde la plus récente, puis **Restaurer celle-ci**.

## Logiciel libre

Dobo est un logiciel libre, sous licence GNU GPL version 3. Son code source est sur GitHub : [github.com/sookera/dobo](https://github.com/sookera/dobo).
