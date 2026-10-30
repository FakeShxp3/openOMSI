#!/usr/bin/env python3
"""Finish 29-language PR #3429 labels in crates/omsi-app/locales/app.yml.
Run from repository root: python3 3429_complete_translations.py
Commit the modified app.yml to feat/passenger-connections-opt-in.
"""
from pathlib import Path
import json

PATH = Path('crates/omsi-app/locales/app.yml')
KEYS = [
    'Connection chance',
    'Optional onward trips in single player only; disabled in multiplayer',
    'Passenger connections between lines',
    'Probability for eligible transfers; default 20%',
]
# Each row is: language code, label, multiplayer help, toggle label, chance help.
ROWS = '''ca|Probabilitat de transbordament|Trajectes de transbordament opcionals només en mode individual; desactivats en multijugador|Transbordaments de passatgers entre línies|Probabilitat dels transbordaments possibles; valor predeterminat: 20 %
ru|Вероятность пересадки|Необязательные поездки с пересадками только в одиночной игре; отключено в сетевой игре|Пересадки пассажиров между маршрутами|Вероятность допустимых пересадок; по умолчанию 20 %
de|Umsteigewahrscheinlichkeit|Optionale Anschlussfahrten nur im Einzelspielermodus; im Mehrspielermodus deaktiviert|Fahrgastumstiege zwischen Linien|Wahrscheinlichkeit für mögliche Umstiege; Standard: 20 %
fr|Probabilité de correspondance|Trajets avec correspondance facultatifs en solo uniquement ; désactivés en multijoueur|Correspondances des passagers entre lignes|Probabilité des correspondances possibles ; valeur par défaut : 20 %
uk|Імовірність пересадки|Необов'язкові поїздки з пересадками лише в одиночній грі; вимкнено в мережевій грі|Пересадки пасажирів між маршрутами|Імовірність доступних пересадок; типово 20 %
be|Імавернасць перасадкі|Неабавязковыя паездкі з перасадкамі толькі ў адзіночнай гульні; адключана ў сеткавай гульні|Перасадкі пасажыраў паміж маршрутамі|Імавернасць магчымых перасадак; стандартна 20 %
kk|Ауысу ықтималдығы|Қосымша ауысып міну сапарлары тек жеке ойында қолжетімді; көп ойыншы режимінде өшірулі|Жолаушылардың бағыттар арасында ауысып мінуі|Мүмкін ауысулардың ықтималдығы; әдепкісі 20 %
pl|Szansa na przesiadkę|Opcjonalne podróże z przesiadką tylko w trybie jednoosobowym; wyłączone w grze wieloosobowej|Przesiadki pasażerów między liniami|Prawdopodobieństwo dostępnych przesiadek; domyślnie 20%
cs|Pravděpodobnost přestupu|Volitelné navazující jízdy pouze pro jednoho hráče; ve hře více hráčů vypnuto|Přestupy cestujících mezi linkami|Pravděpodobnost možných přestupů; výchozí hodnota 20 %
hu|Átszállási esély|Opcionális átszállások csak egyjátékos módban; többjátékos módban kikapcsolva|Utasok átszállása a járatok között|A lehetséges átszállások valószínűsége; alapérték: 20%
es|Probabilidad de transbordo|Viajes opcionales con transbordo solo en modo individual; desactivados en multijugador|Transbordos de pasajeros entre líneas|Probabilidad de transbordos válidos; valor predeterminado: 20 %
pt|Probabilidade de conexão|Viagens com conexão opcionais apenas no modo individual; desativadas no multijogador|Conexões de passageiros entre linhas|Probabilidade para conexões elegíveis; padrão de 20%
pt-pt|Probabilidade de ligação|Viagens com ligação opcionais apenas no modo individual; desativadas no multijogador|Ligações de passageiros entre linhas|Probabilidade para ligações elegíveis; predefinição de 20%
it|Probabilità di coincidenza|Proseguimenti opzionali solo in modalità giocatore singolo; disattivati in multigiocatore|Coincidenze dei passeggeri tra linee|Probabilità di coincidenze possibili; valore predefinito: 20%
nl|Overstapkans|Optionele aansluitende ritten alleen in de singleplayer; uitgeschakeld in multiplayer|Overstappen van passagiers tussen lijnen|Kans op mogelijke overstappen; standaard 20%
tr|Aktarma olasılığı|İsteğe bağlı aktarmalı yolculuklar yalnızca tek oyunculu modda; çok oyunculu modda devre dışı|Yolcuların hatlar arasında aktarma yapması|Uygun aktarmaların olasılığı; varsayılan %20
ja|乗り継ぎ確率|任意の乗り継ぎはシングルプレイ専用です。マルチプレイでは無効になります|乗客の路線間乗り継ぎ|条件を満たす乗り継ぎの確率。既定値は20%
zh|换乘概率|可选的后续换乘仅限单人模式；多人模式下禁用|乘客在不同线路之间换乘|符合条件的换乘概率；默认 20%
hi|स्थानांतरण की संभावना|वैकल्पिक आगे की यात्रा केवल एकल खिलाड़ी मोड में; मल्टीप्लेयर में बंद|यात्रियों का लाइनों के बीच स्थानांतरण|योग्य स्थानांतरण की संभावना; डिफ़ॉल्ट 20%
zh-tw|轉乘機率|可選的後續轉乘僅限單人模式；多人模式中停用|乘客在不同路線之間轉乘|符合條件的轉乘機率；預設為 20%
ko|환승 확률|선택적 연계 탑승은 싱글플레이에서만 가능하며 멀티플레이에서는 비활성화됩니다|노선 간 승객 환승|조건에 맞는 환승 확률. 기본값 20%
th|โอกาสเปลี่ยนสาย|การเดินทางต่อแบบเลือกได้ใช้ได้เฉพาะโหมดผู้เล่นคนเดียว และปิดในโหมดหลายผู้เล่น|การเปลี่ยนสายของผู้โดยสารระหว่างเส้นทาง|โอกาสเปลี่ยนสายเมื่อเข้าเงื่อนไข ค่าเริ่มต้น 20%
vi|Xác suất chuyển tuyến|Chuyến nối tuyến tùy chọn chỉ dùng trong chế độ chơi đơn; tắt ở chế độ nhiều người chơi|Hành khách chuyển giữa các tuyến|Xác suất chuyển tuyến hợp lệ; mặc định 20%
id|Peluang transit|Perjalanan lanjutan opsional hanya dalam mode pemain tunggal; dinonaktifkan dalam multipemain|Transit penumpang antarjalur|Peluang transit yang memenuhi syarat; bawaan 20%
ms|Kebarangkalian pertukaran laluan|Perjalanan sambungan pilihan hanya dalam mod pemain tunggal; dimatikan dalam mod berbilang pemain|Pertukaran penumpang antara laluan|Kebarangkalian pertukaran yang layak; lalai 20%
tl|Tsansa ng paglipat|Opsyonal na kasunod na biyahe sa single-player lamang; hindi pinagana sa multiplayer|Paglipat ng mga pasahero sa pagitan ng mga linya|Tsansa para sa mga puwedeng paglipat; default na 20%
sv|Byteschans|Valfria anslutningsresor endast i enspelarläge; inaktiverat i flerspelarläge|Passagerarbyten mellan linjer|Sannolikhet för möjliga byten; standardvärde 20 %
sr|Verovatnoća presedanja|Opciona presedanja samo u režimu za jednog igrača; isključena u igri za više igrača|Presedanja putnika između linija|Verovatnoća mogućih presedanja; podrazumevano 20 %
ka|გადაჯდომის ალბათობა|არასავალდებულო გადაჯდომით მგზავრობა მხოლოდ ერთმოთამაშიან რეჟიმშია; მრავალმოთამაშიან რეჟიმში გამორთულია|მგზავრების გადაჯდომა ხაზებს შორის|შესაძლო გადაჯდომების ალბათობა; ნაგულისხმევად 20%'''

def main():
    rows = [line.split('|') for line in ROWS.splitlines()]
    assert len(rows) == 29 and all(len(row) == 5 for row in rows)
    assert len({r[0] for r in rows}) == 29
    text = PATH.read_text(encoding='utf-8')
    start = text.find('\n"Connection chance":')
    if start < 0 or not text.endswith('\n'):
        raise RuntimeError('Expected the four connection strings at the end of app.yml')
    current = text[start:]
    if any(current.count('"'+key+'":') != 1 for key in KEYS):
        raise RuntimeError('Connection localization keys changed unexpectedly')
    sections = []
    for idx, key in enumerate(KEYS):
        sections.append('"'+key+'":\n' + ''.join('  '+row[0]+': '+json.dumps(row[idx+1], ensure_ascii=False)+'\n' for row in rows))
    updated = text[:start] + '\n\n' + '\n'.join(sections)
    if updated == text:
        print('No changes needed')
    else:
        PATH.write_text(updated, encoding='utf-8')
        print(f'Completed {len(KEYS)} strings in {len(rows)} languages in {PATH}')

if __name__ == '__main__':
    main()