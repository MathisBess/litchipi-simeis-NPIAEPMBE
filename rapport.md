# Rapport CI/CD 
## Nathan PIVETEAU, Adam EPIARD et Mathis BESSON

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

