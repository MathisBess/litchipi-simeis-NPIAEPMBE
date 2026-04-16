import sys
import time
from sdk import SimeisSDK

class Game:
    def __init__(self, username, ip, port):
        self.sdk = SimeisSDK(username, ip, port)

    def gameloop(self):
        status = self.sdk.get_player_status()
        sta = status["stations"][0]

        # On a besoin de savoir quelle planète miner pour équiper notre vaisseau
        nearest_planet = self.sdk.scan_planets(sta)[0]
        print("Targeting planet", nearest_planet)

        # Si on commence une nouvelle partie, on s'équipe
        if len(status["ships"]) == 0:
            # Acheter un vaisseau
            print("Buying first ship")
            all_ships = self.sdk.shop_list_ship(sta)
            for s in all_ships:
                print(s)
            ship = all_ships[0]
            self.sdk.buy_ship(sta, ship["id"])

            # En fonction de la planète, on achète un module de minage différent
            if nearest_planet["solid"]:
                mod = "Miner"
            else:
                mod = "GasSucker"
            mod = self.sdk.buy_module_on_ship(sta, ship["id"], mod)

            # On embauche du personnel
            operator = self.sdk.hire_crew(sta, "operator")
            self.sdk.assign_crew_to_ship(sta, ship["id"], operator["id"], mod["id"])

            pilot = self.sdk.hire_crew(sta, "pilot")
            self.sdk.assign_crew_to_ship(sta, ship["id"], pilot["id"], "pilot")

            trader = self.sdk.hire_crew(sta, "trader")
            self.sdk.assign_trader_to_station(sta, trader["id"])

        # Si on reprends une partie existante
        # On retourne à la station, on vide tout, avant de repartir
        else:
            ship = status["ships"][0]
            self.sdk.return_station_and_unload_all(sta, ship["id"])

        # Cycle infini
        #     On va à la planète
        #     On mine
        #     On rentre à la station
        #     On répare le vaisseau, on fait le plein
        #     On vends les resources
        while True:
            status = self.sdk.get_player_status()
            print("Current status: {} credits, costs: {}, time left before lost: {} secs".format(
                round(status["money"], 2), round(status["costs"], 2), int(status["money"] / status["costs"]),
            ))
            if status["money"] <= 0:
                print("You lost")
                return

            # On va à la planète
            self.sdk.travel(ship["id"], nearest_planet["position"])

            # On mine
            prices = self.sdk.get_market_prices()
            info = self.sdk.start_extraction(ship["id"])
            stats = info["mining_rate"]
            totpersec = 0
            for res, amnt in stats.items():
                print(f"{res}: {amnt} /sec")
                totpersec += amnt * prices[res]
            print(f"Total: {totpersec} credits / sec")

            # On attends que l'extraction termine
            # Elle se termine automatiquement quand le cargo est plein
            time.sleep(info["time_fill_cargo"])
            self.sdk.wait_until_ship_idle(ship["id"])

            # On retourne à la station, et on décharge le cargo
            self.sdk.return_station_and_unload_all(sta, ship["id"])

            # On vends tout
            cycletot = 0
            for res, amnt in self.sdk.get_station_resources(sta).items():
                if res in [ "Fuel", "Hull" ]:
                    continue
                got = self.sdk.sell_resource(sta, res, amnt)
                print("Sold", amnt, "of", res, "for", got["added_money"], "credits (fees", got["fees"], "credits)")
                cycletot += got["added_money"]

            # On achète du carburant et on fait le plein
            got = self.sdk.buy_fuel_for_refuel(sta, ship["id"])
            cycletot -= got["removed_money"]
            print("Bought", got["added_cargo"], "of Fuel for", got["removed_money"], "credits (fees", got["fees"], "credits)")
            self.sdk.refuel_ship(sta, ship["id"])

            # On achète des plaques de coque, et on répare la coque
            got = self.sdk.buy_hull_for_repair(sta, ship["id"])
            cycletot -= got["removed_money"]
            print("Bought", got["added_cargo"], "of Hull for", got["removed_money"], "credits (fees", got["fees"], "credits)")
            self.sdk.repair_ship(sta, ship["id"])

            # Rebelotte
            print("Total this cycle:", cycletot)
            print("")

if __name__ == "__main__":
    if len(sys.argv[1:]) < 3:
        print("Usage: python3.py ./client.py <username> <IP> <port>")
        sys.exit(1)

    name = sys.argv[1]
    ip = sys.argv[2]
    port = int(sys.argv[3])
    game = Game(name, ip, port)
    game.gameloop()
