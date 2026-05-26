import sys
import time
import uuid
import requests


class TestAPI:
    def __init__(self, pseudo, ip, port):
        self.base_url = f"http://{ip}:{port}"
        self.session = requests.Session()
        
        # Création du joueur et authentification
        res = self.post(f"/player/new/{pseudo}")
        if res.get("error") == "ok":
            self.player_id = res["playerId"]
            self.session.headers.update({"Simeis-Key": res["key"]})
        else:
            raise Exception(f"Erreur d'authentification : {res}")
        
        # Le serveur a besoin d'un court instant pour initialiser le joueur
        time.sleep(1)

    def get(self, endpoint):
        res = self.session.get(self.base_url + endpoint)
        res.raise_for_status()
        data = res.json()
        if "error" in data and data["error"] != "ok":
            raise Exception(f"Erreur API GET {endpoint}: {data['error']}")
        return data

    def post(self, endpoint, data=None):
        res = self.session.post(self.base_url + endpoint, json=data)
        res.raise_for_status()
        data = res.json()
        if "error" in data and data["error"] != "ok":
            raise Exception(f"Erreur API POST {endpoint}: {data['error']}")
        return data

    def get_player_status(self):
        return self.get(f"/player/{self.player_id}")

    def shop_list_ship(self, sta):
        res = self.get(f"/station/{sta}/shipyard/list")
        return res.get("ships", [])

    def buy_ship(self, sta, ship_id):
        return self.post(f"/station/{sta}/shipyard/buy/{ship_id}")

    def buy_module_on_ship(self, sta, ship_id, modtype):
        return self.post(f"/station/{sta}/shop/modules/{ship_id}/buy/{modtype.lower()}")

    def hire_crew(self, sta, crewtype):
        return self.post(f"/station/{sta}/crew/hire/{crewtype.lower()}")

    def assign_crew_to_ship(self, sta, ship_id, crew_id, mod_id):
        if mod_id == "pilot":
            return self.post(f"/station/{sta}/crew/assign/{crew_id}/ship/{ship_id}/pilot")
        else:
            return self.post(f"/station/{sta}/crew/assign/{crew_id}/ship/{ship_id}/{mod_id}")

    def scan_planets(self, sta):
        res = self.post(f"/station/{sta}/scan")
        return res.get("planets", [])

    def travel(self, ship_id, pos):
        x, y, z = pos
        return self.post(f"/ship/{ship_id}/navigate/{x}/{y}/{z}")

    def wait_until_ship_idle(self, ship_id):
        while True:
            time.sleep(0.5)
            res = self.get(f"/ship/{ship_id}")
            if res.get("state") == "Idle":
                break


def run_scenario_1_economy(ip, port):
    print("--- Scénario 1: Economie et Achat de Vaisseau ---")
    pseudo = f"test_{uuid.uuid4().hex[:8]}"
    api = TestAPI(pseudo, ip, port)
    
    # Vérification de l'argent de départ
    status = api.get_player_status()
    initial_money = status["money"]
    print(f"Argent initial: {initial_money}")
    assert initial_money > 0, "Le joueur doit commencer avec de l'argent"
    
    sta = status["stations"][0]
    
    # Achat d'un vaisseau
    ships = api.shop_list_ship(sta)
    assert len(ships) > 0, "Il doit y avoir des vaisseaux en vente"
    
    ship_to_buy = ships[0]
    api.buy_ship(sta, ship_to_buy["id"])
    print("Vaisseau acheté avec succès.")
    
    status_after_ship = api.get_player_status()
    assert len(status_after_ship["ships"]) == 1, "Le joueur doit posséder 1 vaisseau"
    assert status_after_ship["money"] < initial_money, "L'argent doit avoir diminué après l'achat du vaisseau"
    
    # Achat d'un module de minage
    money_before_mod = status_after_ship["money"]
    api.buy_module_on_ship(sta, ship_to_buy["id"], "Miner")
    print("Module de minage acheté avec succès.")
    
    status_after_mod = api.get_player_status()
    assert status_after_mod["money"] < money_before_mod, "L'argent doit encore avoir diminué après l'achat du module"
    print("Scénario 1 OK !\n")


def run_scenario_2_crew(ip, port):
    print("--- Scénario 2: Embauche et Gestion de l'Équipage ---")
    pseudo = f"test_{uuid.uuid4().hex[:8]}"
    api = TestAPI(pseudo, ip, port)
    
    status = api.get_player_status()
    sta = status["stations"][0]
    
    ships = api.shop_list_ship(sta)
    ship_id = ships[0]["id"]
    api.buy_ship(sta, ship_id)
    
    # Embauche d'un pilote
    pilot = api.hire_crew(sta, "pilot")
    assert "id" in pilot, "La transaction doit réussir et le pilote doit avoir un ID"
    print("Pilote embauché avec succès.")
    
    # Assignation du pilote au vaisseau
    api.assign_crew_to_ship(sta, ship_id, pilot["id"], "pilot")
    print("Pilote assigné au vaisseau avec succès.")
    print("Scénario 2 OK !\n")


def run_scenario_3_travel(ip, port):
    print("--- Scénario 3: Scan et Voyage vers une Planète ---")
    pseudo = f"test_{uuid.uuid4().hex[:8]}"
    api = TestAPI(pseudo, ip, port)
    
    status = api.get_player_status()
    sta = status["stations"][0]
    
    ships = api.shop_list_ship(sta)
    ship_id = ships[0]["id"]
    api.buy_ship(sta, ship_id)
    
    pilot = api.hire_crew(sta, "pilot")
    api.assign_crew_to_ship(sta, ship_id, pilot["id"], "pilot")
    
    # Scan des planètes
    planets = api.scan_planets(sta)
    assert len(planets) > 0, "Le scan doit trouver au moins une planète à proximité"
    target_planet = planets[0]
    print(f"Planète détectée en position {target_planet['position']}.")
    
    # Déplacement du vaisseau
    api.travel(ship_id, target_planet["position"])
    print("Déplacement vers la planète initié avec succès.")
    
    # On attend la fin du déplacement
    api.wait_until_ship_idle(ship_id)
    print("Vaisseau arrivé à destination.")
    print("Scénario 3 OK !\n")


if __name__ == "__main__":
    ip_addr = "127.0.0.1"
    port_num = 8080
    
    if len(sys.argv) >= 3:
        ip_addr = sys.argv[1]
        port_num = int(sys.argv[2])
        
    try:
        run_scenario_1_economy(ip_addr, port_num)
        run_scenario_2_crew(ip_addr, port_num)
        run_scenario_3_travel(ip_addr, port_num)
        print("Tous les tests fonctionnels sont passés avec succès !")
        sys.exit(0)
    except AssertionError as err:
        print(f"Échec d'une assertion : {err}")
        sys.exit(1)
    except Exception as err:
        print(f"Erreur inattendue : {err}")
        sys.exit(1)