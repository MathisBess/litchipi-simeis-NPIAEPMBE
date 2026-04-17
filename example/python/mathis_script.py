import sys
import time
import requests



class MathisAPI:

    def __init__(self, pseudo, ip, port):
        self.base_url = f"http://{ip}:{port}"
        self.session = requests.Session()
        self.player_id = None
        self.ship_id = None

   

    def get(self, endpoint):
        response = self.session.get(self.base_url + endpoint)
        response.raise_for_status()
        return response.json()



    def post(self, endpoint, data=None):
        response = self.session.post(self.base_url + endpoint, json=data)
        response.raise_for_status()
        return response.json()



    def authenticate(self, pseudo):
        print(f"Authentification en cours à : {pseudo}")
        res = self.post(f"/player/new/{pseudo}")

        # Si le joueur existe déjà
        if(res.get("error") == "ok"):
            self.player_id = res.get("playerId")
            key = res.get("key")

            #Configure pour toutes les futurs requêtes la clé d'authentification
            self.session.headers.update({"Simeis-Key": key})
            print(f"Authentification réussie pour le joueur {pseudo} (ID: {self.player_id})")
            return True

        else:
            print(f"Erreur d'authentification : {res.get('error')}")
            return False

    def scan_station(self, ID_station):
        print("Scan de la station en cours...")
        res = self.get(f"/station/{ID_station}/can")
        print(f"Scan terminé. Objets détectés : {len(res.get('objects', []))}")
        return res.get("objects", [])

    def scan_ship_station(self, ID_station):
        print("Recherche de vaisseau sur la station...")
        res = self.get(f"/station/{ID_station}/shipyard/list")
        ships = [obj for obj in res.get("ships", [])]
        print(f"Recherche terminée. Vaisseaux détectés : {ships}")
        return ships


if __name__ == "__main__":
    if len(sys.argv) < 4:
        print("Usage: python bot.py <pseudo> <IP> <port>")
        sys.exit(1)

    pseudo, ip, port = sys.argv[1], sys.argv[2], sys.argv[3]
    api = MathisAPI(pseudo, ip, port)
    
    if api.authenticate(pseudo):
        
        print("Récupération du profil joueur...")
        player_data = api.get(f"/player/{api.player_id}")
        
        stations = player_data.get("stations", [])
        if not stations:
            print("Aucune station trouvée pour ce joueur.")
            sys.exit(1)
            
        station_id = stations[0]
        print(f"Station de départ trouvée : {station_id}")
        
        print("--- Test du Chantier Naval ---")
        ships_available = api.scan_ship_station(station_id)
        
        if ships_available:
            first_ship = ships_available[0]
            print(f"Succès ! Le premier vaisseau en vente est le modèle '{first_ship['id']}'")
            print(f"Il coûte {first_ship['price']} crédits et possède {first_ship['cargo_capacity']} places de soute.")
        else:
            print("Le chantier naval est vide !")