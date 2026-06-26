# Rapport CI/CD 
## Nathan PIVETEAU, Adam EPIARD et Mathis BESSON

---

### TP1 - Mise en place de la ci du projet

#### Les ajouts

- Script automatisé pour les dépendances qui crée une pull request quand elle n'existe pas --> à la base on ne vérifiait pas son existence. Pendant la nuit on a donc observé des erreurs étant donné qu'il essayait de créer une nouvelle en doublon. 
- On a séparé les modèles pour les rapports de bugs et les nouvelles fonctionnalités, car pour nous il est important de différencier ces deux types de retours. Cela permet aussi d'aider à définir leur niveau de priorité.
- Notre bot automatisé vérifie les dépendances à intervalles réguliers, mais cela ne garantit pas qu'une nouvelle dépendance ajoutée soit exempte de faille. C'est pourquoi on a ajouté un audit de sécurité automatique.

#### Ce qu'on a pu apprendre

On était déjà très familiers avec la CI, mais en s'interdisant d'utiliser les outils du marketplace, on a pu découvrir de nouvelles manières de faire et de nouveaux outils comme typst.

#### Amélioration à creuser

- Notre ci est très lente car elle recompile à chaque fois. On a commencé à se renseigner sur l'utilisation du cache pour gagner du temps.
- On aimerait automatiser la création des "Releases" sur GitHub pour que le binaire et le manuel soient téléchargeables en un clic dès qu'on sort une version.

---

### TP2 - Qualité, Analyse et Automatisation des branches

#### Les ajouts

- Mise en place de règles de développement soutenues par des protections de branches (fusion refusée sans succès de la CI et sans approbation de revue).
- Ajout d'un workflow de linting sur chaque PR (cargo check, fmt, clippy pour Rust, et un formateur pour Python).
- Auto-fermeture des PRs ciblant `release/x` si la source n'est pas conforme, et script de "propagation" de correctifs créant automatiquement des backports vers les branches de release grâce au label `propagate:release/x`.
- Un script maison qui scanne le code pour vérifier que chaque commentaire `// T0D0` est explicitement lié à une issue GitHub ouverte (ex: `// T0D0 (#15)`), faisant échouer la CI en cas d'issue fermée ou inexistante.
- Déclenchement d'audits (sécurité avec `cargo-audit`, propreté avec `cargo-udeps`) lors des merges vers les branches de release.

#### Ce qu'on a pu apprendre

- Pour respecter notre consigne de ne pas utiliser d'actions tierces du marketplace, nous avons dû manipuler l'API GitHub directement en ligne de commande pour fermer des PRs, vérifier le statut des issues (pour les T0D0s) et créer des PRs de propagation de bugs.
- Nous avons découvert des outils très puissants de l'écosystème Rust, notamment `cargo-udeps` pour traquer les dépendances orphelines et `cargo-clippy` pour imposer de bonnes pratiques.

#### Amélioration à creuser

- Le script de vérification des T0D0s fait actuellement échouer la CI en affichant l'erreur dans les logs. Il serait très intéressant d'utiliser l'API GitHub pour écrire un commentaire automatisé directement sur la ligne de code concernée dans la Pull Request.
- L'outil `cargo-udeps` requiert la toolchain *nightly* et recompile une grande partie du projet pour fonctionner, ce qui ralentit considérablement l'analyse avancée. Mettre en place un système de cache robuste serait ici très bénéfique.

---

### TP3 - Tests Avancés (Fonctionnels, Coverage et Property-Based)

#### Les ajouts

- Création d'un client API Python autonome testant 3 mécaniques clés de Simeis : l'économie, la gestion d'équipage et les déplacements. Le tout s'exécute sur un serveur Rust lancé à la volée en arrière-plan dans la CI.
- Ajout de vérifications mathématiques avancées . Nous l'avons intégré à deux niveaux : une version rapide sur chaque Pull Request (avec annulation dynamique via `concurrency` en cas de nouveau push), et une version longue pour traquer les cas limites avant une Release.
- Intégration de `cargo-tarpaulin`. Si la couverture de code de la PR est inférieure à 50%, un label `not enough tests` est automatiquement appliqué par notre CI grâce à l'outil `gh` (GitHub CLI).

#### Ce qu'on a pu apprendre

- Nous avons appris à exploiter pleinement l'environnement natif d'Ubuntu (`apt-get` pour Python, `curl/tar` pour télécharger les binaires de tarpaulin à la volée) afin de configurer nos workflows manuellement sans dépendre d'actions externes (setup-python, toolchains Rust, etc.).
- On a compris l'importance de séparer l'étape de compilation de l'étape de lancement du serveur (avec `&` et `sleep`) pour éviter les erreurs "Connection refused" et permettre aux scripts de tests Python de communiquer avec succès.

#### Amélioration à creuser

- L'exécution de la version lourde du property-based testing  allonge logiquement le temps de la CI. Il serait intéressant de paralléliser ces exécutions mathématiques sur plusieurs runners.
- Générer des rapports visuels (HTML ou XML) pour le Code Coverage et les tests fonctionnels, afin de les lier à la CI sous forme d'Artifacts pour une consultation plus agréable que la lecture des logs bruts.

---

### TP4 - Optimisations
#### Les ajouts
- Implémentation d'un système de cache sur les workflows. L'objectif de cet ajout est de réduire au maximum le temps d'exécution des différentes étapes pour qu'elles ne durent pas plus de 10 à 30 secondes chacune.
- Parallélisation maximale des jobs au sein du workflow.
- Mise en place d'un mécanisme d'arrêt d'urgence : la CI est désormais configurée pour échouer rapidement en cas d'erreur. Si un job échoue, cela interrompt immédiatement tous les autres jobs parallèles afin que tout s'arrête rapidement.
- Création d'un workflow utilisant une matrice de build, qui se déclenche automatiquement lors d'une tentative de merge d'une Pull Request d'une branche feature/x vers main.

#### Ce qu'on a pu apprendre
- Gérer la charge d'une matrice de build complexe. En croisant 3 systèmes d'exploitation et 4 versions de Rust, la CI génère 12 exécutions simultanées. Nous avons concrètement vu l'intérêt de faire stopper tous les jobs si l'un d'eux échoue afin de ne pas gaspiller de précieuses minutes de calcul sur les runners GitHub.
- Concevoir un système de cache robuste sans recourir à des actions tierces de la communauté. Cela nous a obligés à bien comprendre l'arborescence des fichiers de compilation Rust (target/) et la manière de les sauvegarder et de les restaurer efficacement entre deux exécutions.

#### Amélioration à creuser
- Les performances diffèrent selon les systèmes d'exploitation. Les runners macos-latest et windows-latest ont parfois des temps d'allocation ou de compilation natifs plus longs que Linux. Il serait intéressant de pouvoir ajuster les timeouts spécifiquement en fonction de l'OS cible.
- La gestion de la taille du cache. Si nos étapes sont descendues à 10-30 secondes grâce à lui, ce cache risque de grossir au fil des versions (notamment avec 4 versions de Rust différentes à stocker). Implémenter une stratégie d'invalidation et de nettoyage automatique de ce cache serait un vrai plus pour l'avenir.

---

### TP5 - Artefacts de release et Paquet Debian

#### Les ajouts
- Automatisation complète du processus de publication : lors du merge d'une PR sur une branche `release/x`, un workflow génère et met en ligne une release officielle sur GitHub.
- Génération automatique d'un Changelog structuré en utilisant exclusivement l'outil en ligne de commande `gh`. Les modifications sont intelligemment catégorisées (Features, Bugfixes, Autre) selon la nomenclature des branches sources.
- Création d'un paquet d'installation `.deb` complet pour les environnements Debian/Ubuntu, généré directement depuis la CI. Ce paquet inclut la création d'un utilisateur système dédié, une page de manuel, un service `systemd` pour l'exécution en arrière-plan, et force l'installation d'une dépendance factice (`cmatrix`).

#### Ce qu'on a pu apprendre
- Démystification du packaging Debian : nous avons compris qu'un fichier `.deb` repose avant tout sur la reproduction fidèle de l'arborescence du système cible (ex: placer le binaire dans un sous-dossier virtuel `usr/bin/`).

#### Amélioration à creuser
- Actuellement, le test du paquet Debian généré s'effectue manuellement sur une Machine Virtuelle locale. Il serait très pertinent d'ajouter un job final dans notre CI qui lancerait un conteneur ou une VM éphémère basée sur Debian pour télécharger l'artefact, l'installer via `apt`, et vérifier le statut du service avec `systemctl` automatiquement.

---

### TP6 - Continuous Delivery et Dockerisation

#### Les ajouts
- Conteneurisation de notre serveur Simeis (Rust) via un `Dockerfile` optimisé. La construction de cette image et son envoi (push) vers le registre public Docker Hub sont désormais totalement automatisés dans la CI lors d'une release, en utilisant les secrets GitHub pour l'authentification.
- Conteneurisation de notre client interactif (Python) permettant d'interroger le serveur en lui passant dynamiquement ses arguments de connexion (`<pseudo> <IP> <port>`).
- Création et configuration d'un réseau Docker virtuel (`simeis-network`) en local pour faire communiquer de manière fluide le conteneur du client et celui du serveur.

#### Ce qu'on a pu apprendre
- Nous avons appris qu'un serveur configuré sur l'adresse `127.0.0.1` à l'intérieur d'un conteneur devenait complètement aveugle aux requêtes extérieures. Il a fallu repenser notre configuration d'écoute sur `0.0.0.0` et exploiter le système de résolution DNS interne de Docker pour que le client Python trouve le serveur via son nom de conteneur (`simeis-server`).
- Lors de la création de l'image client, nous avons été confrontés à l'absence de certains `Cargo.toml`. Cela nous a permis de bien comprendre comment définir correctement le "contexte de build" d'une commande Docker pour englober un espace de travail Rust (Workspace) parent dans son intégralité plutôt qu'un sous-dossier isolé.

#### Amélioration à creuser (pas le temps)
- Actuellement, tester notre architecture multi-conteneurs en local demande de saisir manuellement plusieurs commandes consécutives dans le terminal pour créer le réseau, configurer la sécurité et lier les deux conteneurs. Une amélioration simple et très efficace serait de mettre en place un fichier `docker-compose.yml`. Cela permettrait de centraliser la configuration du réseau virtuel, d'intégrer l'option de sécurité nécessaire pour le serveur, et de lancer l'ensemble de notre infrastructure (serveur + client) en une seule ligne de commande (`docker compose up`).

---

### TP7 - Déploiement Continu (CD) sur VPS

#### Les ajouts

- **Automatisation du déploiement (CD)** : Configuration d'un job `deploy` dans notre workflow de publication (`auto-release.yml`). Lors d'une nouvelle release, le workflow télécharge l'artefact du paquet `.deb` généré, prépare la clé privée SSH et l'IP du VPS à partir des secrets GitHub, puis utilise `scp` et `ssh` pour copier le paquet, l'installer (`dpkg -i`) et redémarrer le service `simeis.service` sur la machine distante.
- **Validation automatique en ligne** : Après le déploiement sur le VPS, le workflow monte un tunnel SSH temporaire en tâche de fond (`ssh -L`) pour rediriger le port API sur le runner GitHub, puis effectue une requête `curl /version`. Il extrait ensuite la version JSON via Python et la compare au numéro de version dans `simeis-server/Cargo.toml`. Le job échoue si le serveur est injoignable ou si la version retournée n'est pas celle attendue.

#### Ce qu'on a pu apprendre

- **Configuration sécurisée de SSH en environnement non interactif** : Lors des premiers essais, la CI restait bloquée indéfiniment car SSH attendait une confirmation interactive pour valider l'empreinte du serveur distant. Après des recherches sur StackOverflow et la documentation de SSH, nous avons compris qu'il fallait utiliser l'option `-o StrictHostKeyChecking=no` pour bypasser cette invite manuelle, tout en utilisant `ssh-keyscan` pour enregistrer l'adresse de notre hôte dans les `known_hosts` de la CI.
- **Interrogation sécurisée à travers un tunnel SSH** : Pour des raisons de sécurité, les ports du serveur distant sur le VPS ne sont pas ouverts au grand public sur le web. Pour vérifier si notre déploiement a réussi, nous avons dû apprendre à initier un tunnel SSH local à distance (`-L 8081:localhost:8081`) combiné aux options `-N` (ne pas exécuter de commande) et `-f` (passer SSH en arrière-plan). Cela permet au runner de faire un `curl` sur `localhost:8081` comme s'il était directement sur le VPS.

#### Améliorations à creuser

- **Sécurisation des clés d'hôte (Host Keys)** : Pour le moment, l'usage de `-o StrictHostKeyChecking=no` reste une solution de contournement rapide dans le script de déploiement. Pour renforcer la sécurité et éviter tout risque d'attaque par interception, il serait préférable de configurer proprement la clé publique du VPS à l'avance dans les variables de secrets et de la charger au moment du setup de SSH dans le runner pour pouvoir conserver une vérification stricte.
- **Privilèges sudo non interactifs** : L'installation par `dpkg` et le redémarrage du service nécessitent des privilèges administrateur, que nous passons aujourd'hui en injectant le mot de passe via `echo "$VPS_PASSWORD" | sudo -S`. Il serait beaucoup plus propre de configurer les droits sudo de l'utilisateur de déploiement sur le VPS (via `/etc/sudoers`) pour lui autoriser uniquement ces deux commandes précises sans demande de mot de passe (`NOPASSWD`).