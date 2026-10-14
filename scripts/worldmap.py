#!/usr/bin/env python3
"""worldmap.py REGION OUT -- draw a treadcore map of a piece of the world.

    python3 -I scripts/worldmap.py atlantic maps/world/atlantic.map
    python3 -I scripts/worldmap.py europe   maps/world/europe.map
    make worldmaps

Roadmap step 10g. The coastlines are coarse polygons in longitude and
latitude, typed from an atlas at a degree or so: enough for a game map
of a few hundred tiles, not for navigation. Each edge is roughened by
midpoint displacement from a fixed seed, so a coast reads as a coast and
the output is the same every run. Land is grass, forest by latitude and
noise (none in the deserts), rough ground (rubble) along the mountain
ranges, a few great rivers as water, and the roads as roads, bridged
where they cross water: the Pan-American Highway from Prudhoe Bay to
Ushuaia, broken at the Darien Gap as the real one is, and Europe's trunk
roads from Lisbon to Moscow, with the Channel, the Bosphorus and the
Strait of Gibraltar bridged. Sixteen starts in sixteen cities, a depot
beside each, neutral depots and pillboxes at the chokepoints, and pads
with neutral structures where the industry is: the Gulf Coast, the
North Sea, the Ruhr, Arabia's oilfields, the data centres of Virginia,
Moscow and Riyadh.

The projection is equirectangular about a standard parallel, so a tile
is about the same distance east as north in the middle of the map.
"""

import heapq
import math
import random
import sys

# --------------------------------------------------------------- the world

# Land, as (lon, lat) rings. Water bodies inside land are listed after.
NORTH_AMERICA = [
    (-168, 66), (-162, 70), (-156, 71.3), (-141, 69.6), (-128, 70), (-115, 68.5), (-104, 68),
    (-95, 68.5), (-90, 69), (-85, 69.5), (-80, 68), (-72, 67.5), (-64, 66), (-62, 64), (-64, 60),
    (-61.5, 56.5), (-57, 53), (-55.7, 52.1), (-56, 49.5), (-53, 47.5), (-53.6, 46.6), (-59, 47.6),
    (-61.5, 45.8), (-60, 45.6), (-63.5, 44.5), (-66, 43.8), (-67, 44.8), (-70, 43.7), (-70.5, 41.7),
    (-72, 41.2), (-74, 40.5), (-74.2, 39.4), (-76, 37), (-75.6, 35.2), (-77, 34.5), (-79, 33.2),
    (-81, 31.5), (-81.3, 30), (-80.1, 26.8), (-80.4, 25.2), (-81.2, 25.2), (-81.8, 26.5), (-82.7, 28),
    (-83.7, 29.9), (-85, 29.7), (-86.5, 30.4), (-89, 30.3), (-89.4, 29), (-90.5, 29.1), (-94, 29.6),
    (-97.2, 27.8), (-97.4, 25.5), (-97.7, 22.5), (-97.2, 20.5), (-96, 19), (-94.5, 18.2), (-91.5, 18.4),
    (-90.4, 19.8), (-90.4, 21.1), (-87, 21.5), (-87.4, 19.5), (-88, 18.5), (-88.2, 16.2), (-86.5, 15.8),
    (-84, 15.9), (-83.2, 15), (-83.6, 12), (-83.7, 10.9), (-82, 9.3), (-80, 9.3), (-79, 9.6), (-77.4, 8.6),
    (-77.9, 7.2), (-78.5, 8.0), (-80, 7.3), (-82, 8.2), (-85.7, 10), (-86, 11.5), (-87.5, 13), (-91.5, 13.9),
    (-94, 16), (-96.5, 15.7), (-101, 17.5), (-105.5, 20.5), (-105.7, 23), (-109.9, 22.9), (-112, 24.8),
    (-114.1, 28), (-115.8, 30.4), (-117.1, 32.5), (-118.5, 34), (-120.6, 34.6), (-122.5, 37.7),
    (-124.2, 40.4), (-124.5, 43), (-124, 46.3), (-124.7, 48.4), (-123, 49), (-127.5, 50.8), (-130, 54),
    (-134, 58), (-140, 59.8), (-146, 60.8), (-151.5, 59.3), (-154, 57.5), (-158, 56.5), (-163, 54.8),
    (-158, 58.5), (-162, 59.8), (-165.5, 62), (-164.5, 63.2), (-161, 64.5), (-166, 65.4),
]
BAFFIN = [(-80, 73.5), (-68, 70.5), (-62, 66.5), (-65, 63), (-72, 63.5), (-78, 64.5), (-85, 69), (-90, 72.5)]
GREENLAND = [
    (-73, 78), (-60, 78), (-40, 78), (-20, 78), (-18, 75), (-22, 70.5), (-26, 68.5), (-35, 65.5), (-41, 62.5),
    (-43, 60), (-46, 60.8), (-50, 63), (-52, 66.5), (-54, 69.5), (-56, 72.5), (-62, 76),
]
ICELAND = [(-24, 65.5), (-22, 66.4), (-16, 66.5), (-13.5, 65.2), (-15, 64.3), (-19, 63.4), (-22.5, 63.8)]
CUBA = [(-85, 21.9), (-82, 23.1), (-77.5, 22.2), (-74.2, 20.2), (-77.7, 19.9), (-80.5, 21.7)]
HISPANIOLA = [(-74.5, 18.4), (-72.8, 19.9), (-70, 19.7), (-68.4, 18.6), (-71.5, 17.6), (-74.4, 18.2)]
SOUTH_AMERICA = [
    (-77.4, 8.6), (-75.5, 10.5), (-74, 11.3), (-71.5, 12.4), (-70, 11.5), (-68, 10.5), (-64, 10.6),
    (-61.5, 10.7), (-60, 8.5), (-57, 6), (-53, 5.5), (-51, 4.2), (-50, 1.8), (-49, 0), (-48, -1), (-44, -2.5),
    (-40, -2.9), (-37, -4.8), (-35.2, -5.5), (-34.8, -7.5), (-35.5, -9.5), (-37, -11), (-38.6, -13),
    (-39, -17.5), (-40, -20), (-41, -22), (-43, -23), (-45, -23.8), (-48.5, -26), (-48.6, -28.5),
    (-50.5, -30.5), (-52.5, -33.5), (-54, -34.8), (-56.5, -34.6), (-58.6, -34.3), (-57.5, -36), (-57, -37.5),
    (-58, -38.5), (-62, -39), (-62.3, -40.5), (-65, -41), (-64, -42.5), (-65.5, -45), (-67.5, -46),
    (-66, -47.5), (-68.3, -50.2), (-69, -51.6), (-68.4, -52.4), (-70, -52.9), (-72, -53.6), (-74.5, -52),
    (-75.5, -48.5), (-74, -45), (-73.5, -42), (-73.7, -38), (-71.7, -33), (-71.4, -29), (-70.5, -23),
    (-70.3, -18.4), (-71.5, -17.3), (-76, -14), (-77.2, -12), (-79.2, -7.5), (-81.2, -6), (-80.3, -3.4),
    (-80.9, -1), (-80, 0.8), (-78.9, 1.6), (-77.3, 4), (-77.4, 6.6), (-77.9, 7.2),
]
TIERRA_DEL_FUEGO = [(-68.6, -52.8), (-66.5, -53.6), (-65.2, -54.8), (-67, -55.1), (-68.5, -55.5), (-71.5, -54.6), (-70.5, -53.6), (-69.5, -52.9)]
OLD_WORLD = [
    (-5.6, 36.0), (-8.9, 37.0), (-9.5, 38.7), (-8.9, 41.5), (-9.3, 43.0), (-8, 43.7), (-4, 43.5),
    (-1.8, 43.4), (-1.2, 44.7), (-1.2, 46.2), (-2.5, 47.3), (-4.7, 48.0), (-4.3, 48.7), (-1.6, 48.7),
    (-1.9, 49.7), (0.2, 49.5), (1.6, 50.9), (3.2, 51.4), (4.3, 52.3), (4.8, 53.0), (7, 53.5), (8.6, 53.9),
    (8.6, 55.5), (8.1, 56.8), (10.5, 57.7), (10.3, 56.2), (10.6, 55.0), (9.9, 54.8), (11, 54), (13.8, 54.2),
    (14.3, 53.9), (18.5, 54.8), (21, 55.3), (21, 56.5), (21.7, 57.6), (24.1, 57.0), (23.5, 58.6), (24.8, 59.5),
    (28, 59.5), (30.3, 59.9), (29, 60.2), (25, 60.3), (22.5, 60), (21.3, 61.5), (21.6, 63.3), (25, 65),
    (25.5, 65.8), (22, 65.7), (21, 64.3), (18.5, 63), (17.3, 61.2), (18.9, 59.9), (18.1, 59.3), (16.5, 57),
    (16, 56.2), (14.3, 55.5), (12.9, 55.4), (12.6, 56.2), (11.8, 57.7), (11.2, 58.9), (10.6, 59.9),
    (9.5, 59), (8, 58.1), (6, 58.1), (5.3, 59.3), (5, 61), (5.4, 62.5), (8, 63.5), (10.5, 64.5), (12.5, 66),
    (14, 67.5), (16, 68.5), (19, 70), (23.5, 70.8), (28, 71), (31, 70.3), (33, 69.3), (36, 69.1), (41, 67.7),
    (44, 68.5), (50, 68), (54, 68.6), (59, 68.9), (66, 69.5), (66, 25.2), (62, 25.2), (59, 25.4),
    (57.3, 25.9), (56.4, 26.5), (56.2, 25.6), (56.4, 24.5), (58.8, 23.6), (59.8, 22.4), (57.8, 19), (55, 17),
    (52, 16), (48.5, 14), (45, 12.8), (43.5, 12.7), (43.3, 12.4), (43.1, 11.6), (44, 10.5), (51.2, 11.8),
    (51, 10.4), (49, 6), (46.5, 2.5), (42.5, -0.5), (40, -3), (39.2, -6.5), (40.5, -10.5), (40.6, -15),
    (36.8, -18), (35.3, -22.5), (32.8, -26), (31, -29.5), (27.5, -33.7), (25, -34), (20, -34.8),
    (18.4, -34.2), (17.9, -32.5), (15, -27), (14.4, -22.5), (11.8, -17.2), (13.5, -12), (12.2, -6),
    (9.2, -1), (9.6, 3.5), (8.5, 4.5), (6, 4.3), (3, 6.4), (-2, 4.8), (-7.5, 4.4), (-11, 6.8), (-13.3, 8.5),
    (-15, 11), (-16.8, 13.5), (-17.5, 14.7), (-16.3, 19.5), (-17, 21), (-15, 25), (-13, 27.6), (-9.8, 29.8),
    (-9.6, 32), (-6.8, 34), (-5.9, 35.8),
]
GREAT_BRITAIN = [
    (-5.7, 50.1), (-3, 50.6), (1.4, 51.2), (1.7, 52.7), (0.3, 53.4), (-0.1, 54.4), (-1.6, 55.6), (-2, 56),
    (-1.8, 57.6), (-3, 58.6), (-5, 58.6), (-5.4, 57.5), (-6.2, 56.5), (-5.4, 55.7), (-5, 54.8), (-3.3, 54.9),
    (-3, 53.4), (-4.6, 53.3), (-4.2, 52.4), (-5.3, 51.8), (-3, 51.5), (-4.5, 51.2),
]
IRELAND = [(-6, 52.2), (-6.2, 53.8), (-5.6, 54.6), (-6.3, 55.3), (-8.2, 55.2), (-10, 54.2), (-9.8, 53), (-10.4, 52), (-9.5, 51.6), (-8, 51.7)]
SICILY = [(12.4, 38.1), (15.6, 38.3), (15.1, 36.7), (12.5, 37.6)]
SARDINIA = [(8.2, 41.1), (9.7, 41), (9.6, 39.1), (8.4, 39), (8.4, 40.6)]
CORSICA = [(9.4, 43), (9.5, 41.4), (8.6, 41.8), (8.6, 42.6)]
CRETE = [(23.5, 35.6), (26.3, 35.2), (24.5, 34.9)]
CYPRUS = [(32.3, 35), (34.6, 35.6), (33.8, 34.6), (32.4, 34.7)]
MADAGASCAR = [(49.4, -12), (50.4, -15.5), (47.2, -25), (45, -25.5), (43.3, -22), (44.3, -16.5), (46.5, -15.7)]

LANDS = [NORTH_AMERICA, BAFFIN, GREENLAND, ICELAND, CUBA, HISPANIOLA, SOUTH_AMERICA, TIERRA_DEL_FUEGO,
         OLD_WORLD, GREAT_BRITAIN, IRELAND, MADAGASCAR]
# Islands in the inland seas: drawn after the seas, or the seas take them.
ISLANDS = [SICILY, SARDINIA, CORSICA, CRETE, CYPRUS]

MEDITERRANEAN = [
    (-5.9, 35.9), (-4.5, 36.6), (-2, 36.8), (-0.5, 38.3), (0.2, 38.8), (-0.3, 39.5), (0.9, 41), (3.2, 41.9),
    (3.1, 43.1), (4.5, 43.4), (6.5, 43.1), (8, 43.9), (8.8, 44.4), (10.2, 43.9), (11, 42.5), (12.5, 41.5),
    (14, 40.8), (15.7, 40), (15.7, 38.0), (16.1, 37.9), (17.1, 39), (16.5, 40.2), (18.5, 40.1), (16, 41.5),
    (14, 42.5), (13.5, 43.6), (12.3, 44.5), (12.5, 45.5), (13.7, 45.7), (14.5, 45), (15.5, 43.8), (17.5, 43),
    (19.4, 42), (19.5, 40.5), (20.2, 39.5), (21, 38.3), (22, 37), (22.8, 36.5), (23.2, 38.2), (23, 39.3),
    (22.9, 40.6), (24, 40.9), (26, 40.8), (26.2, 39.5), (26.8, 38.3), (27.3, 37), (28.3, 36.7), (30.5, 36.5),
    (32.5, 36.1), (34.5, 36.7), (36, 36.5), (35.9, 35), (35, 33.2), (34.5, 31.5), (32.3, 31.2), (30, 31.5),
    (28, 31), (25, 31.7), (23, 32.6), (20, 30.9), (19.5, 30.3), (18, 30.8), (15.5, 32.3), (13, 32.9),
    (11, 33.5), (10.2, 34.5), (11, 35.5), (10.2, 37.2), (9.6, 37.3), (8, 36.9), (5, 36.8), (3, 36.8),
    (1, 36.5), (-1, 35.7), (-2.2, 35.1), (-4.5, 35.2), (-5.3, 35.9),
]
BLACK_SEA = [
    (28, 41.6), (28.6, 43.5), (28.9, 44.8), (30.7, 46.5), (32, 46.5), (33.5, 46), (32.5, 45.4), (33.5, 44.5),
    (35, 44.8), (36.5, 45.3), (38, 44.5), (39.7, 43.5), (41.5, 41.5), (39, 41), (36, 41.7), (33, 42), (30, 41.2),
    (29.1, 41.2),
]
CASPIAN = [(46.9, 44.8), (49, 46.5), (51.5, 47), (53, 46.5), (51.3, 45.2), (53, 41), (54, 40), (53.9, 37.3), (51, 36.7), (49, 37.6), (48.9, 38.9), (49.5, 40.3), (48.5, 41.8), (47.5, 43)]
RED_SEA = [(32.6, 29.9), (33.6, 27.3), (35.5, 24), (37.3, 21), (38.6, 18), (40.5, 15), (42.5, 13), (43.4, 12.5), (43.3, 13.5), (42.7, 15.5), (41.5, 17.5), (39.6, 20.5), (38.6, 23.5), (37, 25.5), (35.2, 28.0), (34.6, 29.5), (32.8, 29.9)]
PERSIAN_GULF = [(48, 30), (50, 30.2), (51.5, 27.9), (54.5, 26.6), (56.3, 27), (56.4, 26.3), (54, 24.2), (52, 24), (51.6, 25.8), (51, 26), (50.2, 25.7), (49.5, 27), (48.4, 28.5)]
BALTIC_INNER = [(10.6, 57.5), (10.9, 56.3), (12.3, 56.0), (12.5, 55.2), (11.5, 55.0)]
HUDSON_BAY = [(-94.5, 58.8), (-92.5, 57), (-88, 56.5), (-82.3, 55), (-79.5, 51.5), (-79, 54.8), (-77, 58), (-78, 62.3), (-72, 62), (-65, 61.5), (-70, 61.5), (-78, 62.8), (-83, 64), (-87, 64.3), (-90.7, 63.6), (-94, 61)]
GULF_OF_CALIFORNIA = [(-114.8, 31.7), (-113, 31.2), (-110.8, 27.9), (-109, 25.5), (-108.4, 25.0), (-109.9, 23.2), (-110.3, 24.2), (-112.1, 26.8), (-114.3, 30)]
SUPERIOR = [(-92, 46.7), (-88, 48.9), (-84.7, 46.8), (-87, 46.5)]
MICHIGAN = [(-87.8, 41.7), (-88, 44.5), (-85, 45.8), (-86.3, 42.5)]
WATERS = [MEDITERRANEAN, BLACK_SEA, CASPIAN, RED_SEA, PERSIAN_GULF, BALTIC_INNER, HUDSON_BAY, GULF_OF_CALIFORNIA, SUPERIOR, MICHIGAN]

# Narrow water the polygons are too coarse to open: (from, to), drawn a
# tile wide. The Strait of Magellan, the Bosphorus and Dardanelles, the
# Strait of Gibraltar, the Channel, the Kattegat and Oresund.
STRAITS = [
    ((-74.6, -52.7), (-68.3, -52.55)),
    ((26.2, 40.1), (29.05, 41.25)),
    ((-6.2, 35.95), (-5.2, 35.95)),
    ((-1.5, 50.3), (2.0, 51.05)),
    ((12.6, 56.1), (12.7, 55.3)),
]

RIVERS = [
    [(-95, 47.5), (-93, 45), (-91, 43), (-90.2, 38.6), (-89.5, 36), (-91, 33), (-91.3, 31), (-90, 29.2)],
    [(-73, -4), (-65, -3.5), (-60, -3.1), (-55, -2.2), (-51, -0.8), (-49.5, -0.2)],
    [(9, 48), (13, 48.6), (16.4, 48.2), (19, 47.5), (19, 45.5), (20.5, 44.8), (22.5, 44.3), (25, 43.7), (28, 44.6), (29.6, 45.2)],
    [(7.6, 47.6), (8.2, 49.5), (7.6, 50.4), (7, 50.9), (6.1, 51.9), (4.2, 51.9)],
    [(36, 57), (40, 57.4), (44, 56.3), (49, 55.8), (48.5, 53.2), (46, 48.6), (47.8, 46.2)],
    [(31.3, 30.1), (32.9, 24), (33.5, 19), (32.5, 15.6), (31.6, 9), (31.6, 4)],
    [(-79, 43.6), (-76.5, 44.2), (-74, 45.4), (-71.2, 46.8), (-68, 48.6), (-66, 49.5)],
    [(-58.5, -34.4), (-59, -32.5), (-60.7, -31.6), (-60.5, -27.5), (-57.6, -25.3), (-55, -20)],
]

MOUNTAINS = [
    [(-150, 63), (-140, 61.5), (-131, 58), (-123, 52.5), (-117, 49), (-112, 45), (-108, 42), (-106, 38.5), (-106, 34), (-105, 30), (-103, 25), (-100.5, 21)],
    [(-75, 9.5), (-76.5, 5), (-78, 0), (-79, -4), (-77, -9), (-73, -14), (-69, -18), (-68.5, -24), (-70, -30), (-70.5, -35), (-71.5, -40), (-72.5, -46), (-73, -50)],
    [(-1.5, 42.8), (2.5, 42.6)],
    [(5.8, 44.2), (7, 45.9), (9.5, 46.5), (12, 47), (15, 47.3)],
    [(-9, 30.5), (-5, 32), (-1, 33.5), (4, 35.5), (9, 36)],
    [(59, 68), (59.5, 62), (59, 56), (58.5, 51)],
    [(37.5, 43.5), (41, 42.8), (44, 42.5), (47.5, 41.5)],
    [(29, 37.5), (33, 37.2), (37, 38), (41, 39.5), (44, 39.5)],
    [(16, 49.5), (20, 49.4), (23, 48.5), (25.5, 46.5), (24, 45.5), (21.5, 45)],
    [(10.5, 44.2), (13, 43), (15, 41.3), (16, 39.5)],
]

# Deserts: no forest in these (lon0, lat0, lon1, lat1) boxes.
DESERTS = [(-17, 15, 35, 32), (35, 12, 60, 33), (-120, 24, -102, 37), (-75, -30, -68, -15), (-72, -52, -64, -38), (11, -28, 22, -17), (45, 37, 66, 45)]

# Cities: name -> (lon, lat). Starts, depots, roads and industry refer
# to them by name.
CITY = {
    "prudhoe bay": (-148.4, 70.2), "fairbanks": (-147.7, 64.8), "whitehorse": (-135, 60.7), "dawson creek": (-120.2, 55.8),
    "edmonton": (-113.5, 53.5), "calgary": (-114, 51), "fort mcmurray": (-111.4, 56.7), "great falls": (-111.3, 47.5),
    "denver": (-105, 39.7), "dallas": (-96.8, 32.8), "laredo": (-99.5, 27.5), "monterrey": (-100.3, 25.7),
    "mexico city": (-99.1, 19.4), "tampico": (-97.9, 22.2), "oaxaca": (-96.7, 17.1), "tuxtla": (-93.1, 16.75),
    "guatemala city": (-90.5, 14.6), "san salvador": (-89.2, 13.7), "tegucigalpa": (-87.2, 14.1), "managua": (-86.3, 12.1),
    "san jose": (-84.1, 9.9), "panama city": (-79.5, 9.0), "yaviza": (-77.7, 8.2), "turbo": (-76.7, 8.1),
    "medellin": (-75.6, 6.2), "bogota": (-74.1, 4.7), "cali": (-76.5, 3.4), "quito": (-78.5, -0.2),
    "guayaquil": (-79.9, -2.2), "piura": (-80.6, -5.2), "lima": (-77.0, -12.0), "arequipa": (-71.5, -16.4),
    "arica": (-70.3, -18.5), "antofagasta": (-70.4, -23.6), "santiago": (-70.6, -33.4), "puerto montt": (-72.9, -41.5),
    "mendoza": (-68.8, -32.9), "buenos aires": (-58.4, -34.6), "bahia blanca": (-62.3, -38.7), "comodoro rivadavia": (-67.5, -45.9),
    "rio gallegos": (-69.2, -51.6), "ushuaia": (-68.3, -54.8), "caracas": (-66.9, 10.5), "maracaibo": (-71.6, 10.6),
    "sao paulo": (-46.6, -23.5), "santos": (-46.3, -23.95), "rio de janeiro": (-43.2, -22.9), "brasilia": (-47.9, -15.8),
    "salvador": (-38.5, -13.0), "recife": (-34.9, -8.05), "manaus": (-60.0, -3.1), "asuncion": (-57.6, -25.3),
    "seattle": (-122.3, 47.6), "san francisco": (-122.4, 37.8), "los angeles": (-118.2, 34.0), "el paso": (-106.5, 31.8),
    "houston": (-95.4, 29.8), "new orleans": (-90.0, 30.0), "atlanta": (-84.4, 33.7), "miami": (-80.2, 25.8),
    "washington": (-77.0, 38.9), "ashburn": (-77.5, 39.05), "new york": (-74.0, 40.7), "boston": (-71.0, 42.4),
    "chicago": (-87.6, 41.9), "winnipeg": (-97.1, 49.9), "toronto": (-79.4, 43.7), "montreal": (-73.6, 45.5),
    "halifax": (-63.6, 44.65),
    "lisbon": (-9.1, 38.7), "madrid": (-3.7, 40.4), "bordeaux": (-0.6, 44.8), "barcelona": (2.2, 41.4),
    "marseille": (5.4, 43.3), "lyon": (4.8, 45.8), "paris": (2.35, 48.9), "calais": (1.85, 50.95), "dover": (1.3, 51.1),
    "london": (-0.13, 51.5), "edinburgh": (-3.2, 55.95), "aberdeen": (-2.1, 57.15), "brussels": (4.35, 50.85),
    "rotterdam": (4.5, 51.9), "essen": (7.0, 51.45), "cologne": (6.96, 50.94), "hamburg": (10.0, 53.55),
    "berlin": (13.4, 52.5), "copenhagen": (12.6, 55.7), "malmo": (13.0, 55.6), "stockholm": (18.1, 59.3),
    "oslo": (10.75, 59.9), "stavanger": (5.7, 58.97), "helsinki": (24.9, 60.2), "warsaw": (21.0, 52.2), "belchatow": (19.3, 51.3),
    "minsk": (27.6, 53.9), "moscow": (37.6, 55.75), "st petersburg": (30.3, 59.9), "kyiv": (30.5, 50.45),
    "odesa": (30.7, 46.5), "zaporizhzhia": (35.1, 47.8), "milan": (9.2, 45.5), "rome": (12.5, 41.9),
    "naples": (14.3, 40.85), "munich": (11.6, 48.1), "vienna": (16.4, 48.2), "budapest": (19.0, 47.5),
    "belgrade": (20.5, 44.8), "sofia": (23.3, 42.7), "athens": (23.7, 38.0), "istanbul": (29.0, 41.0),
    "ankara": (32.9, 39.9), "damascus": (36.3, 33.5), "amman": (35.9, 31.95), "baghdad": (44.4, 33.3),
    "riyadh": (46.7, 24.7), "dhahran": (50.1, 26.3), "tarifa": (-5.6, 36.0), "tangier": (-5.8, 35.75),
    "rabat": (-6.8, 34.0), "algiers": (3.05, 36.75), "hassi messaoud": (6.0, 31.7), "tunis": (10.2, 36.8),
    "tripoli": (13.2, 32.9), "ras lanuf": (18.6, 30.5), "cairo": (31.2, 30.0), "suez": (32.55, 29.97),
    "dakar": (-17.4, 14.7), "lagos": (3.4, 6.5), "reykjavik": (-21.9, 64.15), "kazan": (49.1, 55.8),
}

PAN_AMERICAN_NORTH = ["prudhoe bay", "fairbanks", "whitehorse", "dawson creek", "edmonton", "calgary", "great falls", "denver", "dallas", "laredo", "monterrey", "mexico city", "oaxaca", "tuxtla", "guatemala city", "san salvador", "tegucigalpa", "managua", "san jose", "panama city", "yaviza"]
PAN_AMERICAN_SOUTH = ["turbo", "medellin", "bogota", "cali", "quito", "guayaquil", "piura", "lima", "arequipa", "arica", "antofagasta", "santiago", "mendoza", "buenos aires", "bahia blanca", "comodoro rivadavia", "rio gallegos", "ushuaia"]

ROADS_AMERICAS = [
    PAN_AMERICAN_NORTH, PAN_AMERICAN_SOUTH,
    ["edmonton", "fort mcmurray"], ["santiago", "puerto montt"], ["bogota", "caracas"], ["caracas", "maracaibo"],
    ["seattle", "san francisco", "los angeles", "el paso", "houston", "new orleans", "atlanta", "washington", "new york", "boston"],
    ["new york", "chicago", "denver"], ["atlanta", "miami"], ["houston", "dallas"], ["washington", "ashburn"],
    ["calgary", "winnipeg", "toronto", "montreal", "halifax"], ["toronto", "new york"], ["monterrey", "tampico"],
    ["buenos aires", "asuncion"], ["buenos aires", "sao paulo", "rio de janeiro", "salvador", "recife"], ["sao paulo", "brasilia", "manaus"], ["sao paulo", "santos"],
]
ROADS_OLD_WORLD = [
    ["lisbon", "madrid", "bordeaux", "paris"], ["madrid", "barcelona", "marseille", "lyon", "paris"],
    ["london", "dover", "calais", "paris"], ["london", "edinburgh", "aberdeen"],
    ["paris", "brussels", "cologne", "essen", "hamburg", "copenhagen", "malmo", "stockholm"], ["brussels", "rotterdam"], ["cologne", "berlin"],
    ["malmo", "oslo", "stavanger"], ["berlin", "warsaw", "minsk", "moscow"], ["moscow", "st petersburg", "helsinki"], ["moscow", "kazan"],
    ["warsaw", "belchatow"], ["warsaw", "kyiv", "zaporizhzhia"], ["kyiv", "odesa"], ["moscow", "kyiv"],
    ["lyon", "milan", "rome", "naples"], ["milan", "munich", "berlin"], ["munich", "vienna", "budapest", "belgrade", "sofia", "istanbul", "ankara"],
    ["sofia", "athens"], ["ankara", "damascus", "amman", "riyadh", "dhahran"], ["damascus", "baghdad", "riyadh"],
    ["tarifa", "tangier", "rabat"], ["madrid", "tarifa"], ["tangier", "algiers", "tunis", "tripoli", "ras lanuf", "cairo", "suez", "amman"],
    ["algiers", "hassi messaoud"], ["rabat", "dakar"], ["dakar", "lagos"],
]

# Industry: a zone of pads at a city with the structures on it.
INDUSTRY = [
    ("prudhoe bay", ["refinery"]), ("fort mcmurray", ["refinery", "power"]), ("houston", ["refinery", "power"]),
    ("tampico", ["refinery"]), ("maracaibo", ["refinery", "power"]), ("santos", ["refinery", "power"]),
    ("ashburn", ["datacentre", "power"]), ("chicago", ["power", "workshop"]), ("los angeles", ["datacentre", "power"]),
    ("aberdeen", ["refinery"]), ("stavanger", ["refinery", "power"]), ("rotterdam", ["refinery", "power"]),
    ("essen", ["power", "workshop"]), ("belchatow", ["power"]), ("zaporizhzhia", ["power", "workshop"]),
    ("moscow", ["datacentre", "power"]), ("kazan", ["refinery", "power"]), ("reykjavik", ["datacentre", "power"]),
    ("dhahran", ["refinery", "power"]), ("riyadh", ["datacentre", "power"]), ("hassi messaoud", ["refinery", "power"]),
    ("ras lanuf", ["refinery"]), ("lagos", ["refinery", "power"]), ("paris", ["datacentre", "power"]),
    ("buenos aires", ["power", "workshop"]), ("mexico city", ["power", "workshop"]),
]

# Chokepoints: neutral pillboxes.
GUNS = ["panama city", "yaviza", "turbo", "tarifa", "tangier", "istanbul", "suez", "dover", "calais", "copenhagen", "laredo", "rio gallegos", "dawson creek", "amman", "belgrade", "kyiv"]

REGIONS = {
    "atlantic": {
        "bbox": (-170, -57, 66, 76), "width": 512, "parallel": 30, "seed": 0x5EA,
        "starts": ["new york", "berlin", "buenos aires", "riyadh", "los angeles", "madrid", "lima", "moscow",
                   "mexico city", "london", "sao paulo", "istanbul", "edmonton", "rome", "bogota", "cairo"],
        "depots": ["panama city", "chicago", "houston", "santiago", "manaus", "dakar", "lagos", "kyiv", "oslo", "algiers", "baghdad", "reykjavik"],
    },
    "europe": {
        "bbox": (-12, 33, 46, 64), "width": 320, "parallel": 48, "seed": 0xE0,
        "starts": ["london", "moscow", "madrid", "istanbul", "paris", "berlin", "rome", "kyiv",
                   "lisbon", "stockholm", "warsaw", "athens", "edinburgh", "vienna", "oslo", "algiers"],
        "depots": ["brussels", "milan", "budapest", "minsk", "copenhagen", "marseille", "belgrade", "tunis", "st petersburg", "ankara", "hamburg", "odesa"],
    },
}

GLYPH = {"grass": ".", "forest": "T", "road": "=", "water": "~", "bridge": "+", "rubble": "%", "depot": "D", "pad": "_"}

# ------------------------------------------------------------- the drawing


class Canvas:
    def __init__(self, region):
        r = REGIONS[region]
        self.lon0, self.lat0, self.lon1, self.lat1 = r["bbox"]
        self.k = math.cos(math.radians(r["parallel"]))
        self.w = r["width"]
        self.h = int(round(self.w * (self.lat1 - self.lat0) / ((self.lon1 - self.lon0) * self.k)))
        self.cells = [["water"] * self.w for _ in range(self.h)]
        self.rng = random.Random(r["seed"])

    def xy(self, lon, lat):
        x = (lon - self.lon0) / (self.lon1 - self.lon0) * self.w
        y = (self.lat1 - lat) / (self.lat1 - self.lat0) * self.h
        return x, y

    def inside(self, x, y):
        return 0 <= x < self.w and 0 <= y < self.h

    def rough(self, ring, depth=3):
        """Midpoint displacement on every edge: a coast, not a polygon."""
        pts = [self.xy(*p) for p in ring]
        for _ in range(depth):
            out = []
            for i, a in enumerate(pts):
                b = pts[(i + 1) % len(pts)]
                out.append(a)
                d = math.hypot(b[0] - a[0], b[1] - a[1])
                if d < 1.5:
                    continue
                nx, ny = -(b[1] - a[1]) / d, (b[0] - a[0]) / d
                s = self.rng.uniform(-0.18, 0.18) * d
                out.append(((a[0] + b[0]) / 2 + nx * s, (a[1] + b[1]) / 2 + ny * s))
            pts = out
        return pts

    def fill(self, ring, what, depth=3):
        """Scanline fill of a ring: the centre of each tile tested."""
        pts = self.rough(ring, depth)
        n = len(pts)
        for y in range(self.h):
            cy = y + 0.5
            xs = []
            for i in range(n):
                (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % n]
                if (y0 <= cy < y1) or (y1 <= cy < y0):
                    xs.append(x0 + (cy - y0) / (y1 - y0) * (x1 - x0))
            xs.sort()
            for a, b in zip(xs[0::2], xs[1::2]):
                for x in range(max(0, int(math.ceil(a - 0.5))), min(self.w, int(math.floor(b - 0.5)) + 1)):
                    self.cells[y][x] = what

    def line(self, a, b, step=0.4):
        """The tiles along a segment in map coordinates, 4-connected."""
        (x0, y0), (x1, y1) = a, b
        n = max(1, int(math.hypot(x1 - x0, y1 - y0) / step))
        out = []
        last = None
        for i in range(n + 1):
            t = i / n
            x, y = int(x0 + (x1 - x0) * t), int(y0 + (y1 - y0) * t)
            if last and abs(x - last[0]) + abs(y - last[1]) == 2:
                out.append((x, last[1]))  # no diagonal steps: the router will not take them
            if (x, y) != last:
                out.append((x, y))
                last = (x, y)
        return [p for p in out if self.inside(*p)]

    def path(self, lonlats, what, keep=()):
        pts = [self.xy(*p) for p in lonlats]
        for a, b in zip(pts, pts[1:]):
            for x, y in self.line(a, b):
                if self.cells[y][x] not in keep:
                    self.cells[y][x] = what

    # What a road costs to lay through each ground, per tile: an old
    # road is reused, water is bridged only where the way round is long.
    ROAD_COST = {"road": 0.35, "bridge": 0.5, "grass": 1.0, "pad": 1.0, "depot": 1.0, "forest": 1.6, "rubble": 2.5, "water": 30.0}

    def route(self, a, b):
        """A* over the tiles, four ways, for a road from a to b."""
        (ax, ay), (bx, by) = a, b
        # A pull toward the straight line, so a road over open country
        # runs the way it would be surveyed, not in a staircase.
        L = math.hypot(bx - ax, by - ay) or 1.0
        off = lambda q: abs((bx - ax) * (ay - q[1]) - (ax - q[0]) * (by - ay)) / L
        best = {a: 0.0}
        came = {}
        todo = [(0.0, a)]
        while todo:
            _, p = heapq.heappop(todo)
            if p == b:
                break
            x, y = p
            for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                if not self.inside(*q):
                    continue
                g = best[p] + self.ROAD_COST[self.cells[q[1]][q[0]]] + 0.12 * min(off(q), 12.0)
                if g < best.get(q, 1e18):
                    best[q] = g
                    came[q] = p
                    heapq.heappush(todo, (g + 0.35 * (abs(q[0] - bx) + abs(q[1] - by)), q))
        path = [b]
        while path[-1] != a and path[-1] in came:
            path.append(came[path[-1]])
        return path[::-1]

    def road(self, names):
        pts = []
        for n in names:
            x, y = self.xy(*CITY[n])
            pts.append(self.land_near(int(x), int(y)) or (int(x), int(y)))
        for a, b in zip(pts, pts[1:]):
            for x, y in self.route(a, b):
                c = self.cells[y][x]
                if c == "water":
                    self.cells[y][x] = "bridge"
                elif c not in ("bridge", "depot", "pad"):
                    self.cells[y][x] = "road"

    def land_near(self, x, y, r=6):
        best = None
        for dy in range(-r, r + 1):
            for dx in range(-r, r + 1):
                px, py = x + dx, y + dy
                if self.inside(px, py) and self.cells[py][px] not in ("water", "bridge"):
                    d = dx * dx + dy * dy
                    if best is None or d < best[0]:
                        best = (d, px, py)
        return None if best is None else (best[1], best[2])


def noise(seed):
    """Value noise on a 32-lattice, two octaves, for the forests."""
    rng = random.Random(seed)
    lat = [[rng.random() for _ in range(33)] for _ in range(33)]

    def raw(x, y):
        x0, y0 = int(math.floor(x)) % 32, int(math.floor(y)) % 32
        fx, fy = x - math.floor(x), y - math.floor(y)
        s = lambda t: t * t * (3 - 2 * t)
        a = lat[y0][x0] + (lat[y0][x0 + 1] - lat[y0][x0]) * s(fx)
        b = lat[y0 + 1][x0] + (lat[y0 + 1][x0 + 1] - lat[y0 + 1][x0]) * s(fx)
        return a + (b - a) * s(fy)

    return lambda x, y: raw(x, y) * 0.65 + raw(x * 2.3, y * 2.3) * 0.35


def forest_line(a):
    """The noise a tile must beat to be forest, by latitude: thick in
    the tropics and the boreal belt, light between, none at the pole."""
    pts = [(0, 0.40), (10, 0.42), (22, 0.62), (40, 0.62), (52, 0.48), (64, 0.5), (72, 0.85), (90, 1.0)]
    for (a0, w0), (a1, w1) in zip(pts, pts[1:]):
        if a <= a1:
            return w0 + (w1 - w0) * (a - a0) / (a1 - a0)
    return 1.0


def draw(region):
    c = Canvas(region)
    r = REGIONS[region]
    for ring in LANDS:
        c.fill(ring, "grass")
    for ring in WATERS:
        c.fill(ring, "water", depth=2)
    for ring in ISLANDS:
        c.fill(ring, "grass", depth=2)
    # Forest: how much by latitude (boreal and tropical thick, the
    # temperate belt light), none in the deserts.
    n = noise(r["seed"] + 1)
    for y in range(c.h):
        lat = c.lat1 - (y + 0.5) / c.h * (c.lat1 - c.lat0)
        for x in range(c.w):
            if c.cells[y][x] != "grass":
                continue
            lon = c.lon0 + (x + 0.5) / c.w * (c.lon1 - c.lon0)
            if any(a <= lon <= b and p <= lat <= q for a, p, b, q in DESERTS):
                continue
            want = forest_line(abs(lat))
            if n(x * 24.0 / c.w * (c.w / 128.0) * 0.5, y * 24.0 / c.h * (c.h / 128.0) * 0.5) > want:
                c.cells[y][x] = "forest"
    for ridge in MOUNTAINS:
        c.path(ridge, "rubble", keep=("water",))
        # Two tiles wide, a little ragged.
        pts = [(lo + 0.25 * (c.lon1 - c.lon0) / c.w, la - 0.25 * (c.lat1 - c.lat0) / c.h) for lo, la in ridge]
        c.path(pts, "rubble", keep=("water",))
    for river in RIVERS:
        c.path(river, "water")
    for a, b in STRAITS:
        c.path([a, b], "water")
    # The Darien Gap: jungle, and no road through it.
    gap = [c.xy(-77.7, 8.4), c.xy(-76.7, 7.6)]
    for y in range(int(min(gap[0][1], gap[1][1])) - 2, int(max(gap[0][1], gap[1][1])) + 3):
        for x in range(int(gap[0][0]) - 1, int(gap[1][0]) + 2):
            if c.inside(x, y) and c.cells[y][x] in ("grass", "rubble"):
                c.cells[y][x] = "forest"
    for road in ROADS_AMERICAS + ROADS_OLD_WORLD:
        if all(c.inside(*c.xy(*CITY[n])) for n in road):
            c.road(road)
        else:
            # The part of a road on this map: the cities that are on it.
            on = [n for n in road if c.inside(*c.xy(*CITY[n]))]
            if len(on) >= 2:
                c.road(on)

    places = []

    def site(name):
        x, y = c.xy(*CITY[name])
        p = c.land_near(int(x), int(y))
        if p is None:
            raise SystemExit("worldmap: %s is not near land on the %s map" % (name, region))
        return p

    starts = [site(s) for s in r["starts"]]
    # Industry first, so a depot or a start does not land on a pad.
    structures = []
    for city, kinds in INDUSTRY:
        if not c.inside(*c.xy(*CITY[city])):
            continue
        x, y = site(city)
        # A zone two tiles off the city, toward the side with land.
        best = None
        for ox, oy in ((2, 2), (-4, 2), (2, -4), (-4, -4), (3, 0), (-5, 0), (0, 3), (0, -5)):
            zx, zy = x + ox, y + oy
            tiles = [(zx + i, zy + j) for i in range(3) for j in range(3)]
            dry = sum(1 for tx, ty in tiles if c.inside(tx, ty) and c.cells[ty][tx] not in ("water", "bridge"))
            if best is None or dry > best[0]:
                best = (dry, zx, zy)
        _, zx, zy = best
        for i in range(3):
            for j in range(3):
                if c.inside(zx + i, zy + j):
                    c.cells[zy + j][zx + i] = "pad"
        for k, kind in enumerate(kinds):
            sx, sy = zx + (0 if k == 0 else 2), zy + (0 if k == 0 else 2)
            structures.append((sx, sy, kind, city))
    depots = []
    for i, s in enumerate(starts):
        # A depot two tiles from the start, on land, not on the start.
        x, y = s
        for ox, oy in ((2, 0), (-2, 0), (0, 2), (0, -2), (2, 2), (-2, -2), (3, 1), (-3, -1)):
            if c.inside(x + ox, y + oy) and c.cells[y + oy][x + ox] not in ("water", "bridge", "pad"):
                depots.append((x + ox, y + oy, r["starts"][i], True))
                break
    for name in r["depots"]:
        if c.inside(*c.xy(*CITY[name])):
            x, y = site(name)
            if c.cells[y][x] == "pad":
                y += 3
            depots.append((x, y, name, False))
    for x, y, _, _ in depots:
        c.cells[y][x] = "depot"
    guns = []
    for name in GUNS:
        if not c.inside(*c.xy(*CITY[name])):
            continue
        x, y = site(name)
        # Beside the road, not on the city: two tiles off where it is dry.
        for ox, oy in ((1, 1), (-1, 1), (1, -1), (-1, -1), (2, 0), (0, 2)):
            px, py = x + ox, y + oy
            if c.inside(px, py) and c.cells[py][px] in ("grass", "forest", "rubble", "road") and all(abs(px - sx) + abs(py - sy) > 2 for sx, sy in starts):
                if c.cells[py][px] == "forest":
                    c.cells[py][px] = "grass"
                guns.append((px, py, name))
                break
    # Clear forest off starts so nobody begins inside a wood.
    for x, y in starts:
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                if c.inside(x + dx, y + dy) and c.cells[y + dy][x + dx] == "forest":
                    c.cells[y + dy][x + dx] = "grass"
    return c, starts, depots, guns, structures


def write(region, out):
    c, starts, depots, guns, structures = draw(region)
    r = REGIONS[region]
    lines = ["treadcore map 1", "size %d %d" % (c.w, c.h), "seed %#x" % r["seed"]]
    lines.append("# %s, drawn by scripts/worldmap.py (roadmap step 10g): coarse coastlines in" % region)
    lines.append("# longitude and latitude, %s to %s E, %s to %s N, about %d degrees a tile" % (c.lon0, c.lon1, c.lat0, c.lat1, round(1 / (c.w / (c.lon1 - c.lon0)))))
    lines.append("# rows: . grass  T forest  = road  ~ water  + bridge  % rough ground  D depot  _ pad")
    for row in c.cells:
        lines.append("".join(GLYPH[t] for t in row))
    lines.append("# starts, in seat order")
    for x, y in starts:
        lines.append("start %s %s" % (x + 0.5, y + 0.5))
    lines.append("# depots: a start's own first, then neutral ones")
    for x, y, name, own in depots:
        lines.append("depot %s %s%s" % (x + 0.5, y + 0.5, "" if own else " neutral"))
    lines.append("# pillboxes at the chokepoints: " + ", ".join(g[2] for g in guns))
    for x, y, _ in guns:
        lines.append("pillbox %s %s neutral" % (x + 0.5, y + 0.5))
    lines.append("# industry: " + ", ".join(sorted(set(s[3] for s in structures))))
    for x, y, kind, _ in structures:
        lines.append("structure %s %s %s neutral" % (x + 0.5, y + 0.5, kind))
    lines.append("# seats: " + ", ".join("%d %s" % (i, n) for i, n in enumerate(r["starts"])))
    with open(out, "w") as f:
        f.write("\n".join(lines) + "\n")
    print("worldmap: %s -> %s (%d x %d, %d starts, %d depots, %d pillboxes, %d structures)" % (region, out, c.w, c.h, len(starts), len(depots), len(guns), len(structures)))


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] not in REGIONS:
        raise SystemExit("usage: worldmap.py {%s} OUT.map" % "|".join(REGIONS))
    write(sys.argv[1], sys.argv[2])
