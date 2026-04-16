import os
import sys
import math
import time
import json
import string
import urllib.request

class SimeisError(Exception):
    pass

def get_dist(a, b):
    return math.sqrt(((a[0]-b[0]) ** 2) + ((a[1]-b[1]) ** 2) + ((a[2]-b[2]) ** 2))

# Check if types are present in the list
def check_has(alld, key, *req):
    alltypes = [c[key] for c in alld.values()]
    return all([k in alltypes for k in req])

class SimeisSDK:
    def __init__(self, username, ip, port):
        self.url = f"http://{ip}:{port}"
        assert self.api("/ping")["ping"] == "pong"
        self.setup_player(username)

    def api(self, path, method="GET", timeout=5, **qry):
        print(method, path)

        tail = ""
        if len(qry) > 0:
            tail += "?"
            tail += "&".join([
                "{}={}".format(k, urllib.parse.quote(v)) for k, v in qry.items()
            ])

        qry = f"{self.url}{path}{tail}"

        hdr = {}
        if hasattr(self, "player"):
            hdr["Simeis-Key"] = self.player["key"]
        req = urllib.request.Request(qry, headers=hdr, method=method)

        reply = urllib.request.urlopen(req, timeout=timeout)

        data = json.loads(reply.read().decode())
        err = data.pop("error")
        if err != "ok":
            raise SimeisError(err)

        return data

    def get(self, *args, **kwargs):
        return self.api(*args, method="GET", **kwargs)

    def post(self, *args, **kwargs):
        return self.api(*args, method="POST", **kwargs)
        
    # If we have a file containing the player ID and key, use it
    # If not, create a new player
    # If the player has lost, print an error message
    def setup_player(self, username, force_register=False):
        # Sanitize the username, remove any symbols
        username = "".join([c for c in username if c in string.ascii_letters + string.digits]).lower()

        # If we don't have any existing account
        if force_register or not os.path.isfile(f"./{username}.json"):
            player = self.post(f"/player/new/{username}")
            with open(f"./{username}.json", "w") as f:
                json.dump(player, f, indent=2)       
            self.player = player

        # If an account already exists
        else:
            with open(f"./{username}.json", "r") as f:
                self.player = json.load(f)

        # Try to get the profile
        try:
            player = self.get("/player/{}".format(self.player["playerId"]))

        # If we fail, that must be that the player doesn't exist on the server
        except SimeisError:
            # And so we retry but forcing to register a new account
            return self.setup_player(username, force_register=True)

        # If the player already failed, we must reset the server
        # Or recreate an account with a new nickname
        if player["money"] <= 0.0:
            print("!!! Player already lost, please restart the server to reset the game")
            sys.exit(0)

    def get_player_status(self):
        return self.get("/player/" + str(self.player["playerId"]))

    def get_ship_status(self, ship_id):
        return self.get(f"/ship/{ship_id}")

    def get_station_status(self, sta):
        return self.get(f"/station/{sta}")

    def shop_list_modules(self, sta):
        all = self.get(f"/station/{sta}/shop/modules")
        return sorted(all, key = lambda mod: mod["price"])

    def shop_list_ship(self, sta):
        all = self.get(f"/station/{sta}/shipyard/list")["ships"]
        return sorted(all, key = lambda ship: ship["price"])

    def buy_ship(self, sta, shipid):
        return self.post(f"/station/{sta}/shipyard/buy/{shipid}")

    def buy_module_on_ship(self, sta, shipid, modtype):
        return self.post(f"/station/{sta}/shop/modules/{shipid}/buy/{modtype}")

    def hire_crew(self, sta, crewtype):
        return self.post(f"/station/{sta}/crew/hire/{crewtype.lower()}")

    def assign_crew_to_ship(self, sta, shipid, operator_id, role):
        return self.post(f"/station/{sta}/crew/assign/{operator_id}/ship/{shipid}/{role}")

    def station_has_trader(self, sta):
        station = self.get(f"/station/{sta}")
        return check_has(station["crew"], "member_type", "Trader")

    def assign_trader_to_station(self, sta, trader_id):
        return self.post(f"/station/{sta}/crew/assign/{trader_id}/trading")

    def compute_travel_cost(self, ship_id, position):
        x, y, z = position
        return self.get(f"/ship/{ship_id}/travelcost/{x}/{y}/{z}")

    def travel(self, ship_id, position, wait_end=True):
        x, y, z = position
        costs = self.post(f"/ship/{ship_id}/navigate/{x}/{y}/{z}")
        if wait_end:
            time.sleep(costs["duration"])
            self.wait_until_ship_idle(ship_id)

    def wait_until_ship_idle(self, ship_id, ts=1):
        ship = self.get(f"/ship/{ship_id}")
        while ship["state"] != "Idle":
            time.sleep(ts)
            ship = self.get(f"/ship/{ship_id}")

    def buy_hull_for_repair(self, sta, ship_id):
        ship = self.get(f"/ship/{ship_id}")
        req = int(ship["hull_decay"])
        # Pas besoin
        if req == 0:
            return None

        cargo = self.get(f"/station/{sta}")["cargo"]
        if "Hull" not in cargo["resources"]:
            cargo["resources"]["Hull"] = 0

        if cargo["resources"]["Hull"] < req:
            need = req - cargo["resources"]["Hull"]
            return self.post(f"/market/{sta}/buy/hull/{need}")
        return None

    def repair_ship(self, sta, ship_id):
        ship = self.get(f"/ship/{ship_id}")
        req = int(ship["hull_decay"])

        # Pas besoin
        if req == 0:
            return None

        cargo = self.get(f"/station/{sta}")["cargo"]
        if "Hull" not in cargo["resources"]:
            cargo["resources"]["Hull"] = 0

        if cargo["resources"]["Hull"] > 0:
            return self.post(f"/station/{sta}/repair/{ship_id}")

        return None

    def buy_fuel_for_refuel(self, sta, ship_id):
        ship = self.get(f"/ship/{ship_id}")
        req = int(ship["fuel_tank_capacity"] - ship["fuel_tank"])

        # Pas besoin
        if req == 0:
            return

        cargo = self.get(f"/station/{sta}")["cargo"]
        if "Fuel" not in cargo["resources"]:
            cargo["resources"]["Fuel"] = 0

        if cargo["resources"]["Fuel"] < req:
            need = req - cargo["resources"]["Fuel"]
            return self.post(f"/market/{sta}/buy/fuel/{need}")
        return None

    def refuel_ship(self, sta, ship_id):
        ship = self.get(f"/ship/{ship_id}")
        req = int(ship["fuel_tank_capacity"] - ship["fuel_tank"])

        # Pas besoin
        if req == 0:
            return

        cargo = self.get(f"/station/{sta}")["cargo"]
        if "Fuel" not in cargo["resources"]:
            cargo["resources"]["Fuel"] = 0        

        if cargo["resources"]["Fuel"] > 0:
            return self.post(f"/station/{sta}/refuel/{ship_id}")
        return None

    def scan_planets(self, sta):
        station = self.get(f"/station/{sta}")
        planets = self.post(f"/station/{sta}/scan")["planets"]
        return  sorted(planets,
            key=lambda pla: get_dist(station["position"], pla["position"])
        )

    def start_extraction(self, ship_id):
        return self.post(f"/ship/{ship_id}/extraction/start")

    # TODO (#33) Unload
    # TODO (#33) Unload_all
    def return_station_and_unload_all(self, sta, ship_id):
        ship = self.get(f"/ship/{ship_id}")
        station = self.get(f"/station/{sta}")
        if ship["position"] != station["position"]:
            self.travel(ship["id"], station["position"])
        return self.post(f"/ship/{ship_id}/unload/{sta}/all")

    def get_station_resources(self, sta):
        return self.get(f"/station/{sta}")["cargo"]["resources"]

    def get_market_prices(self):
        return self.get("/market/prices")

    def sell_resource(self, sta, res, amnt):
        return self.post(f"/market/{sta}/sell/{res}/{amnt}")

    def buy_resource(self, sta, res, amnt):
        return self.post(f"/market/{sta}/buy/{res}/{amnt}")

    # TODO (#33) get_syslogs
    # TODO (#33) Add resources info
    # TODO (#33) Get ship wages cost
    # TODO (#33) Industry
