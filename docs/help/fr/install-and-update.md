# Installer et mettre à jour

Obtenir Dobo, le garder à jour, et le retirer si besoin. Dobo fonctionne sous Windows 10 et 11.

## Dobo-Setup ou Dobo portable

Dobo existe en deux versions, sur [dobo.sookera.com](https://dobo.sookera.com). C’est le même programme, et les deux se mettent à jour de la même façon.

- **Dobo-Setup.exe** installe Dobo pour vous seul (par utilisateur), sans droits d’administrateur, dans `%LOCALAPPDATA%\Programs\Dobo`. Il ajoute Dobo au menu Démarrer (et au bureau si vous voulez) et à la liste des applications de Windows, pour le désinstaller comme n’importe quelle appli.
- **Dobo.exe** est la version portable : un seul fichier. Mettez-le dans un dossier à lui et lancez-le. Il n’est ajouté ni au menu Démarrer ni à la liste des applications.

**Paramètres › À propos** indique laquelle vous avez, **Installée** ou **Portable**, à côté de la version, et où sont ses notes.

### La fenêtre d’installation

La fenêtre d’installation s’ouvre en anglais pour une première installation, et dans la langue de Dobo par-dessus un Dobo installé (une mise à jour, une réinstallation). En haut à droite, **English | Français** : cliquez sur **Français** (ou Tab, puis les flèches) et tous ses mots passent en français d’un coup, sans perdre ce que vous avez déjà réglé. Voici ce qu’elle dit alors :

- **Installer dans** : le dossier, `%LOCALAPPDATA%\Programs\Dobo` au départ. **Modifier** en choisit un autre. Ce doit être un dossier que vous pouvez modifier sans droits d’administrateur (pas Program Files), car Dobo s’y met à jour lui-même : sinon, le programme d’installation dit pourquoi et bloque le bouton jusqu’à ce que vous en choisissiez un autre.
- **Raccourci sur le Bureau** : activé au départ.
- **Démarrer Dobo avec Windows** : activé au départ. Les rappels ne sonnent que si Dobo tourne.
- Cliquez sur **Installer**. Installer vaut acceptation des conditions d’utilisation de Dobo (« En installant, vous acceptez les conditions d’utilisation. »), en lien sous le bouton : le Dobo installé ne les redemande donc pas. À la fin, **Ouvrir Dobo** le lance.

Si vous y choisissez une langue, Dobo s’ouvre ensuite dans celle-ci, sa fenêtre d’accueil aussi ; changez-la quand vous voulez dans **Paramètres › Apparence › Langue**. Si vous n’y touchez pas, Dobo garde sa propre langue.

Si Dobo est ouvert, le programme d’installation le ferme d’abord : vos notes sont enregistrées au fil de l’écriture, rien n’est perdu.

### Réinstaller après une désinstallation

Le programme d’installation se souvient du dossier où Dobo était installé. Si vous avez désinstallé Dobo en gardant vos notes, une nouvelle installation propose ce même dossier sous **Installer dans** (vous pouvez toujours le changer avec **Modifier**), et vos notes sont là, comme vous les avez laissées. Si vous avez désinstallé avec **Supprimer aussi mes notes** activé, ce dossier est oublié lui aussi, et l’installation revient à `%LOCALAPPDATA%\Programs\Dobo`.

### Dobo portable

Au premier lancement, Dobo.exe crée juste à côté un dossier `data` et y garde tout : notes, images, sauvegardes, et les données de sa fenêtre. Déplacez ou copiez le dossier entier (`Dobo.exe` et `data` ensemble) pour l’emporter, sur une clé USB par exemple.

Deux cas où il utilise plutôt le dossier habituel de votre profil (`%APPDATA%\com.sookera.ztodo`) :

- **Ce PC a déjà des notes Dobo à cet endroit** (d’un Dobo installé, ou d’une copie précédente) et le dossier `data` n’en a pas encore : le Dobo portable continue de les utiliser, pour que vos notes ne soient jamais laissées derrière ni coupées en deux. **Paramètres › À propos** dit « Portable, avec les notes déjà présentes sur cet ordinateur. » Un dossier `data` qui contient déjà des notes est toujours celui utilisé.
- **Le dossier de Dobo.exe n’accepte pas l’écriture** (dans Program Files, ou sur un disque en lecture seule) : Dobo demande d’abord, jamais en silence, pour que vos notes ne soient jamais coupées en deux. Son écran de démarrage dit où seraient les notes et pourquoi elles ne peuvent pas y aller, puis propose **Utiliser le dossier habituel** (avec le chemin de ce dossier : vos notes y restent ensuite) ou **Quitter Dobo**. Il redemande à chaque démarrage tant que le dossier n’accepte pas l’écriture, sauf si vos notes sont déjà dans le dossier habituel. Une fois qu’elles y sont, Dobo continue d’utiliser ce dossier, même quand le sien accepte de nouveau l’écriture : rien n’est déplacé sans un mot. **Paramètres › À propos** dit alors « Portable, mais le dossier de Dobo.exe n’accepte pas l’écriture : les notes sont dans votre profil utilisateur. » Dobo ne peut pas non plus s’y mettre à jour : quittez-le, et déplacez le dossier de Dobo là où vous pouvez modifier des fichiers, comme Documents.

Toute personne qui peut ouvrir le dossier portable peut lire les notes qu’il contient : gardez-le dans un dossier à vous.

### Les conditions d’utilisation, au premier lancement

Le Dobo portable affiche ses conditions d’utilisation à son premier lancement : « Avant de commencer, lisez les conditions d’utilisation de Dobo. Dobo démarre dès que vous les acceptez. » Cliquez sur **Accepter et continuer**, ou sur **Quitter Dobo**. Il ne le demande qu’une fois, jamais plus ensuite, même après une mise à jour. Tant que vous n’avez pas accepté, Dobo ne va pas du tout en ligne. Les conditions s’affichent en français, en traduction de courtoisie ; c’est la version anglaise qui fait foi, et le lien **Lire l’original en anglais** l’affiche.

Un Dobo installé ne les demande pas, puisque vous les avez acceptées dans la fenêtre d’installation, sauf après une installation silencieuse (voir plus bas), que personne n’a vue. Si vous utilisiez Dobo avant qu’il ait des conditions, il les affiche une fois aussi (« Dobo a maintenant des conditions d’utilisation »), et vos notes restent telles quelles.

## « Windows a protégé votre ordinateur »

La première fois que vous lancez Dobo-Setup.exe ou Dobo.exe, Windows peut dire qu’il a protégé votre ordinateur. Cliquez sur **Informations complémentaires**, puis **Exécuter quand même**.

La fenêtre de Dobo utilise Microsoft Edge WebView2, déjà présent sur Windows 11 et la plupart des PC sous Windows 10.

## Mises à jour

Ouvrez **Paramètres › Mises à jour**, la deuxième partie des Paramètres, juste après **Apparence** :

- La première ligne nomme votre copie, « Dobo 1.2.0 · Installé » (ou « · Portable »), et dit dessous où en sont les choses : « Pas encore de recherche. » ou « Dernière recherche : … », « Vous avez la dernière version. », « Recherche des mises à jour… », ou qu’une nouvelle version est disponible. Avec la recherche automatique activée, elle ajoute « Dobo vérifie une fois par jour. » Son bouton, **Rechercher les mises à jour**, cherche tout de suite, même avec la recherche automatique désactivée. Quand une nouvelle version est téléchargée et vérifiée, la ligne dit « Dobo 1.2.1 est prêt. Vous l’aurez à la prochaine ouverture de Dobo. », avec dessous ce qu’apporte la nouvelle version, et **Redémarrer maintenant** remplace le bouton. Le tableau le dit aussi une fois, avec **Redémarrer maintenant**. Quand quelque chose ne va pas (pas de connexion, un dossier que Dobo ne peut pas modifier), la raison s’affiche sous la ligne.
- **Chercher les mises à jour automatiquement** (activé au départ) : « Une nouvelle version est téléchargée et vérifiée, puis utilisée au prochain démarrage de Dobo. Vos notes ne sont jamais touchées. » Dobo cherche une fois par jour, aussi s’il reste ouvert des jours dans la zone de notification. S’il n’arrive pas à joindre internet, il réessaie une heure plus tard.
- **Nouveautés** : **Lire** montre ce qu’apporte cette version de Dobo. Elles s’affichent aussi d’elles-mêmes, une seule fois, à la première ouverture de Dobo après une mise à jour : chaque version depuis celle que vous aviez, de la plus récente à la plus ancienne (« Nouveautés depuis Dobo 1.1.0 »), avec des liens vers les guides sur dobo.sookera.com. Une nouvelle installation n’affiche rien. **J’ai compris** ou **‹ Retour** revient là d’où vous venez ; **×** ou **Échap** ramène directement au tableau. Elles sont aussi dans le menu **?** en bas à droite du tableau.

Comment se passe une mise à jour :

- Dobo télécharge la nouvelle version et la vérifie avant tout : elle doit être signée par Sookera, avec la bonne taille et la bonne somme de contrôle. Tout le reste est refusé, et les Paramètres disent pourquoi.
- Elle est utilisée au prochain démarrage de Dobo, ou tout de suite avec **Redémarrer maintenant**.
- La nouvelle version démarre d’abord à l’essai. Si elle ne démarre pas correctement, le démarrage suivant remet la version que vous aviez, qui continue de marcher, et les Paramètres le disent (« Dobo 1.1.1 n’a pas démarré correctement : Dobo 1.1.0 a été remis. »). La recherche automatique ne retélécharge pas cette version ; **Rechercher les mises à jour** le peut.
- Seul Dobo.exe change. Vos notes, vos réglages et vos styles ne sont jamais touchés.
- La première fois qu’une nouvelle version ouvre vos notes, avant d’y ajouter quoi que ce soit, elle les sauvegarde telles qu’elles étaient : **Paramètres › Sauvegardes** range cette sauvegarde sous **Avant une mise à jour**, avec le numéro de la version (les 3 dernières sont gardées). Dobo la fait à son premier démarrage après la mise à jour (le programme d’installation la lui laisse), une seule fois par version, et vite : les notes sont copiées tout de suite ; sur le même lecteur, les images ne sont pas recopiées mais liées (un fichier d’image ne change jamais) ; sinon (un autre lecteur, ou une clé USB qui ne sait pas lier les fichiers), elles sont copiées en arrière-plan une fois le tableau ouvert, et si Dobo est fermé avant, son démarrage suivant termine la copie. Si le dossier des sauvegardes que vous avez choisi ne peut pas servir à temps (un lecteur réseau absent, un lecteur lent ou plein, un dossier qui ne peut pas être modifié), cette sauvegarde va dans le dossier `backups` du dossier des données : **Paramètres › Sauvegardes** dit où et pourquoi, et la range avec les autres (« dans le dossier des données ») tant qu’elle est gardée.

Avec la recherche automatique désactivée, Dobo ne va jamais en ligne pour les mises à jour, sauf si vous cliquez sur **Rechercher les mises à jour**.

Une version de développement ou de test de Dobo (construite à partir de son code source, pas téléchargée sur dobo.sookera.com) ne se met jamais à jour : **Rechercher les mises à jour** dit « Cette copie de Dobo est une version de développement ou de test : elle ne se met jamais à jour. »

### Un Dobo-Setup plus récent sur un Dobo installé

Vous pouvez aussi lancer un Dobo-Setup.exe plus récent sur le Dobo installé : sa fenêtre dit « Mettre à jour Dobo », et **Mettre à jour** le met à jour dans son dossier en gardant vos notes, vos réglages, vos raccourcis et l’épingle de la barre des tâches.

## Installer sans fenêtre

Pour installer sur plusieurs PC, ou depuis un script :

- `Dobo-Setup.exe /S` installe en silence, dans `%LOCALAPPDATA%\Programs\Dobo` (ou dans le dossier où Dobo était installé avant), avec un raccourci sur le bureau.
- `Dobo-Setup.exe /S /D=C:\Outils\Dobo` installe dans un autre dossier. `/D=` vient en dernier, sans guillemets, même si le chemin a des espaces.
- Le dossier doit être modifiable par cet utilisateur Windows. Sinon, l’installation s’arrête avant de rien changer, avec le code de sortie 2 et la raison dans la console.
- Une installation silencieuse n’accepte pas les conditions à votre place : Dobo les affiche à son premier lancement. Elle ne règle pas non plus **Démarrer Dobo avec Windows** : sur un PC sans notes Dobo, la mise en route de Dobo au premier lancement le propose ; sinon, activez-le dans **Paramètres › Démarrage**.
- `uninstall.exe /S`, dans le dossier de Dobo, désinstalle en silence. Il garde toujours vos notes : **Supprimer aussi mes notes** ne s’applique jamais à une désinstallation silencieuse.

## Désinstaller

### Désinstaller le Dobo installé

La fenêtre de désinstallation parle la langue de Dobo : celle de l’installation, puis, si vous la changez dans **Paramètres › Apparence › Langue**, la nouvelle (avec **Comme Windows**, celle de Windows). Un Dobo installé avec un Dobo-Setup plus ancien, en anglais seulement, se désinstalle en anglais, jusqu’à ce qu’un Dobo à jour ait démarré une fois, ou qu’un Dobo-Setup plus récent soit lancé par-dessus.

1. Ouvrez **Paramètres › Applications › Applications installées** de Windows, trouvez Dobo et cliquez sur **Désinstaller**.
2. Activez **Supprimer aussi mes notes** seulement si vous voulez aussi les effacer. Dessous : « Vos notes, images, paramètres et les sauvegardes du dossier de Dobo. Un dossier de sauvegardes que vous avez choisi est gardé. C’est définitif. » C’est désactivé au départ, pour qu’une réinstallation reprenne où vous en étiez. Si vous pourriez vouloir vos notes un jour, gardez d’abord une sauvegarde hors du dossier des données (voir [Sauvegardes et données](backups-and-data.md)). Un dossier de sauvegardes choisi dans **Paramètres › Sauvegardes** n’est jamais supprimé par la désinstallation : supprimez-le vous-même si vous le souhaitez.
3. Confirmez. Dobo, ses raccourcis et son lancement avec Windows sont retirés : « Dobo n’est plus sur ce PC. »

Si un autre Dobo est ouvert, ou si un Dobo portable utilise les mêmes notes (un portable qui a gardé les notes déjà présentes sur ce PC : il laisse un petit fichier `.dobo-portable-copies` dans ce dossier ; un portable sur une clé USB qui n’est pas branchée compte aussi, pendant 180 jours après son dernier démarrage), vos notes sont gardées même avec **Supprimer aussi mes notes** activé : « Vos notes ont été gardées : un autre Dobo est ouvert, ou un Dobo portable les utilise aussi. » Fermez tous les Dobo, puis supprimez vous-même le dossier `com.sookera.ztodo` de `%APPDATA%` si vous voulez qu’elles disparaissent.

### Retirer le Dobo portable

1. Dans Dobo, désactivez **Paramètres › Démarrage › Démarrer Dobo avec Windows**.
2. Quittez Dobo (clic droit sur l’icône › **Quitter Dobo**).
3. Supprimez Dobo.exe et son dossier `data`. Si **Paramètres › À propos** indiquait que ses notes étaient dans votre profil utilisateur, elles sont dans `%APPDATA%\com.sookera.ztodo` (partagé avec un Dobo installé, si vous en avez un). Un dossier de sauvegardes que vous avez choisi reste tant que vous ne le supprimez pas.

## Du portable à l’installé

Le Dobo installé garde ses notes dans votre dossier utilisateur, pas à côté du portable. Pour emporter vos notes :

1. Dans le Dobo portable, ouvrez **Paramètres › Sauvegardes** et cliquez sur **Sauvegarder**.
2. Quittez-le, puis installez Dobo avec Dobo-Setup.exe, dans un autre dossier que celui du portable (le programme d’installation en demande un autre, pour que les notes du portable restent avec lui).
3. Dans le Dobo installé, ouvrez **Paramètres › Sauvegardes › Dossier › Choisir…** et prenez le dossier `data\backups` du portable.
4. Cliquez sur **Restaurer…** à côté de la sauvegarde la plus récente, puis **Restaurer celle-ci**.

## Les boutons des notifications

Les boutons des notifications de Dobo (**Fait**, **10 min**, **Plus tard…**) ouvrent un lien `ztodo:` que Windows transmet à Dobo. Le Dobo installé l’enregistre toujours. Un Dobo portable ne le fait que si aucun Dobo ne l’a encore, ou si le Dobo qui l’a n’existe plus : lancer une fois une copie portable ne prend jamais les boutons du Dobo installé.

## Logiciel libre

Dobo est un logiciel libre, sous licence GNU GPL version 3. Son code source est sur GitHub : [github.com/sookera/dobo](https://github.com/sookera/dobo).
