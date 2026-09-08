/**
 * In-memory mock backend used in a plain browser (dev server, screenshots, tests).
 * URL params: ?theme=dark|light|system  ?lang=ru|kk|en  ?empty=1 (no roots)  ?indexing=1 (start a simulated run)
 */
import type { Backend } from './backend';
import type {
  AppInfo, DriveInfo, FacetCount, FileCategory, IndexProgress, IndexStats, Preview, PreviewSegment,
  SearchHit, SearchMode, SearchRequest, SearchResponse, Settings, Theme, UiLanguage
} from './types';

export interface MockDoc {
  path: string;
  category: FileCategory;
  size: number;
  modified: number;
  content: string | null;
  encoding: string | null;
}

const ROOT_RU = 'D:\\Документы';
const ROOT_KK = 'D:\\Жұмыс';
const ROOT_EN = 'C:\\Users\\Aslan\\Documents';

const NOW = Math.floor(Date.now() / 1000);
const day = 86400;

function d(path: string, category: FileCategory, size: number, daysAgo: number, content: string | null, encoding: string | null = 'UTF-8'): MockDoc {
  return { path, category, size, modified: NOW - Math.round(daysAgo * day), content, encoding: content ? encoding : null };
}

export const SAMPLE_DOCS: MockDoc[] = [
  d(`${ROOT_RU}\\Договоры\\Договор аренды №12.docx`, 'document', 148_233, 3.2,
    `ДОГОВОР АРЕНДЫ НЕЖИЛОГО ПОМЕЩЕНИЯ № 12\nг. Алматы, 12 марта 2024 г.\n\nТОО «Стройинвест», именуемое в дальнейшем «Арендодатель», в лице директора Ахметова А. С., действующего на основании Устава, с одной стороны, и ИП «Нурлан», именуемый в дальнейшем «Арендатор», с другой стороны, заключили настоящий договор аренды о нижеследующем.\n\n1. ПРЕДМЕТ ДОГОВОРА\n1.1. Арендодатель передаёт, а Арендатор принимает во временное владение и пользование нежилое помещение общей площадью 84,5 кв. м, расположенное по адресу: г. Алматы, ул. Абая, 150, офис 312.\n1.2. Срок аренды составляет 12 месяцев с момента подписания договора.\n\n2. АРЕНДНАЯ ПЛАТА\n2.1. Размер арендной платы составляет 450 000 (четыреста пятьдесят тысяч) тенге в месяц.\n2.2. Оплата производится ежемесячно не позднее 5-го числа текущего месяца.\n\n3. ОТВЕТСТВЕННОСТЬ СТОРОН\n3.1. За просрочку платежа Арендатор уплачивает пеню в размере 0,1 % от суммы задолженности за каждый день просрочки.\n\nПодписи сторон:\nАрендодатель ____________\nАрендатор ____________`),
  d(`${ROOT_RU}\\Договоры\\Договор аренды №7 (2023).docx`, 'document', 132_010, 402,
    `ДОГОВОР АРЕНДЫ № 7\nг. Астана, 1 февраля 2023 г.\n\nНастоящий договор аренды заключён между ТОО «Казтранс» (Арендодатель) и ТОО «Береке» (Арендатор).\nПредмет договора: аренда складского помещения площадью 320 кв. м по адресу: пр. Республики, 24.\nСрок аренды — 24 месяца. Арендная плата — 980 000 тенге в месяц, включая НДС.\nДополнительное соглашение к договору аренды оформляется в письменной форме.`),
  d(`${ROOT_RU}\\Договоры\\Дополнительное соглашение к договору аренды.docx`, 'document', 54_120, 20,
    `ДОПОЛНИТЕЛЬНОЕ СОГЛАШЕНИЕ № 1\nк Договору аренды № 12 от 12.03.2024\n\nСтороны договорились изменить п. 2.1 договора аренды и установить размер арендной платы 470 000 тенге в месяц начиная с 1 сентября 2024 года.\nОстальные условия договора аренды остаются без изменений.`),
  d(`${ROOT_RU}\\Договоры\\Акт приёма-передачи помещения.docx`, 'document', 38_400, 3.1,
    `АКТ ПРИЁМА-ПЕРЕДАЧИ\nк договору аренды № 12\n\nАрендодатель передал, а Арендатор принял помещение по адресу ул. Абая, 150, офис 312. Состояние помещения удовлетворительное, замечаний нет.\nПоказания счётчиков: электроэнергия — 14 520 кВт·ч, вода — 312 м³.`),
  d(`${ROOT_RU}\\Договоры\\Договор поставки оборудования.docx`, 'document', 210_884, 88,
    `ДОГОВОР ПОСТАВКИ № 45-П\nПоставщик обязуется передать Покупателю оборудование согласно спецификации (Приложение 1), а Покупатель — принять и оплатить его.\nСумма договора: 12 400 000 тенге. Срок поставки: 30 календарных дней.\nГарантийный срок на оборудование — 24 месяца.`),
  d(`${ROOT_RU}\\Договоры\\Реестр договоров 2024.xlsx`, 'spreadsheet', 88_912, 1.5,
    `№	Контрагент	Тип	Дата	Сумма\n12	ИП Нурлан	Договор аренды	12.03.2024	450 000\n45-П	ТОО Техносервис	Договор поставки	15.06.2024	12 400 000\n7	ТОО Береке	Договор аренды	01.02.2023	980 000\n3-У	ООО Вектор	Договор оказания услуг	20.01.2024	1 200 000`),
  d(`${ROOT_RU}\\Бухгалтерия\\Счета\\Счёт на оплату №118.pdf`, 'pdf', 96_331, 12,
    `СЧЁТ НА ОПЛАТУ № 118 от 26 августа 2024 г.\nПоставщик: ТОО «Стройинвест», БИН 021240001234\nПокупатель: ИП «Нурлан»\nНаименование: Арендная плата по договору аренды № 12 за сентябрь 2024\nСумма: 470 000,00 тенге\nВсего к оплате: четыреста семьдесят тысяч тенге 00 тиын`),
  d(`${ROOT_RU}\\Бухгалтерия\\Отчёты\\Квартальный отчёт Q2 2024.xlsx`, 'spreadsheet', 412_005, 60,
    `Отчёт о доходах и расходах за 2 квартал 2024 года\nСтатья	Апрель	Май	Июнь\nВыручка	5 200 000	5 480 000	6 010 000\nАренда офиса	450 000	450 000	450 000\nЗаработная плата	2 100 000	2 100 000	2 250 000\nПрибыль	1 320 000	1 560 000	1 890 000`),
  d(`${ROOT_RU}\\Бухгалтерия\\Отчёты\\Годовой отчёт 2023.pdf`, 'pdf', 2_845_112, 240,
    `ГОДОВОЙ ОТЧЁТ ЗА 2023 ГОД\nТОО «Стройинвест»\n\nОсновные показатели: выручка выросла на 18 % по сравнению с 2022 годом. Расходы на аренду помещений составили 11,7 млн тенге. Численность сотрудников — 46 человек.\nВ 2024 году планируется открытие филиала в г. Шымкент.`),
  d(`${ROOT_RU}\\Кадры\\Трудовой договор — Сериков Д.docx`, 'document', 76_540, 150,
    `ТРУДОВОЙ ДОГОВОР № 27\nРаботодатель: ТОО «Стройинвест». Работник: Сериков Даулет Ерланович.\nДолжность: инженер-сметчик. Место работы: г. Алматы, ул. Абая, 150.\nЗаработная плата: 380 000 тенге. Испытательный срок: 3 месяца. Договор заключён на неопределённый срок.`),
  d(`${ROOT_RU}\\Кадры\\Приказ о приёме на работу.docx`, 'document', 22_140, 149,
    `ПРИКАЗ № 27-к\nО приёме на работу\nПринять Серикова Даулета Ерлановича на должность инженера-сметчика с 10 апреля 2024 года с окладом согласно штатному расписанию.\nОснование: трудовой договор № 27.`),
  d(`${ROOT_RU}\\Кадры\\Штатное расписание.xlsx`, 'spreadsheet', 45_330, 30,
    `Штатное расписание на 2024 год\nДолжность	Кол-во	Оклад\nДиректор	1	900 000\nГлавный бухгалтер	1	600 000\nИнженер-сметчик	3	380 000\nМенеджер по аренде	2	320 000\nОфис-менеджер	1	250 000`),
  d(`${ROOT_RU}\\Презентации\\Стратегия развития 2025.pptx`, 'presentation', 5_120_400, 9,
    `Стратегия развития компании на 2025 год\n\nСлайд 2. Цели: рост выручки на 25 %, выход на рынок Узбекистана, цифровизация процессов.\nСлайд 5. Аренда и логистика: оптимизация расходов на аренду складов, переход на долгосрочные договоры аренды.\nСлайд 9. Команда: найм 12 специалистов, программа обучения.`),
  d(`${ROOT_RU}\\Презентации\\Итоги года — совещание.pptx`, 'presentation', 3_308_912, 250,
    `Итоги 2023 года\nКлючевые проекты: ЖК «Алатау», бизнес-центр «Нурлы», реконструкция склада.\nФинансы: выручка 61 млн тенге, EBITDA 14 млн.\nПланы: расширение отдела аренды коммерческой недвижимости.`),
  d(`${ROOT_RU}\\Заметки\\todo.txt`, 'text', 1_204, 0.3,
    `- позвонить арендатору по договору № 12 (продление)\n- отправить счёт № 118\n- согласовать штатное расписание\n- купить бумагу для принтера\n- обновить QIDIR до новой версии`, 'Windows-1251'),
  d(`${ROOT_RU}\\Заметки\\Протокол совещания 05.09.txt`, 'text', 3_870, 3,
    `Протокол совещания от 5 сентября\nПрисутствовали: Ахметов, Серикова, Нурлан, Ким.\nРешили:\n1. Продлить договор аренды офиса на ул. Абая ещё на год.\n2. Закупить оборудование по договору поставки № 45-П до конца месяца.\n3. Подготовить презентацию стратегии к 20 сентября.`),
  d(`${ROOT_RU}\\Книги\\Гражданский кодекс РК (общая часть).epub`, 'ebook', 1_902_331, 700,
    `Глава 29. Имущественный наём (аренда)\nСтатья 540. Договор имущественного найма\n1. По договору имущественного найма (аренды) наймодатель обязуется предоставить нанимателю имущество за плату во временное владение и пользование.\nСтатья 541. Объекты имущественного найма.`),
  d(`${ROOT_RU}\\Архив\\Скан договора 2019.pdf`, 'pdf', 4_410_000, 1800, null),
  d(`${ROOT_RU}\\Архив\\Старые фото\\IMG_2019.zip`, 'other', 184_000_000, 1900, null),
  d(`${ROOT_RU}\\Почта\\Ответ арендодателя.eml`, 'email', 18_220, 4,
    `From: a.akhmetov@stroyinvest.kz\nTo: nurlan@mail.kz\nSubject: Re: Продление договора аренды\n\nДобрый день! Подтверждаем готовность продлить договор аренды № 12 на прежних условиях. Дополнительное соглашение направим до пятницы.\nС уважением, Ахметов А. С.`),

  d(`${ROOT_KK}\\Есептер\\2023 жылғы есеп.xlsx`, 'spreadsheet', 356_120, 245,
    `2023 жылғы қаржылық есеп\nКөрсеткіш	1 тоқсан	2 тоқсан	3 тоқсан	4 тоқсан\nТабыс	12 400 000	13 100 000	14 800 000	15 200 000\nЖалдау ақысы (аренда)	1 350 000	1 350 000	1 350 000	1 350 000\nЕңбекақы	6 300 000	6 300 000	6 750 000	6 750 000\nТаза пайда	2 100 000	2 450 000	3 100 000	3 300 000`),
  d(`${ROOT_KK}\\Есептер\\Тоқсандық есеп 2024 Q1.docx`, 'document', 124_800, 140,
    `2024 жылдың 1 тоқсанындағы қызмет туралы есеп\n\nЕсепті кезеңде компания 3 жаңа келісімшарт жасады, оның ішінде кеңсе үй-жайын жалдау шарты (договор аренды) № 12.\nЖалпы табыс 16,2 млн теңгені құрады, бұл өткен жылдың сәйкес кезеңімен салыстырғанда 9 % артық.\nҚызметкерлер саны — 48 адам.`),
  d(`${ROOT_KK}\\Келісімшарттар\\Жалдау шарты № 5.docx`, 'document', 98_300, 33,
    `ЖАЛДАУ ШАРТЫ № 5\nАлматы қ., 2024 жылғы 5 тамыз\n\n«Стройинвест» ЖШС (Жалға беруші) және «Береке» ЖШС (Жалға алушы) осы жалдау шартын жасады.\n1. Шарттың мәні: Абай даңғылы, 150 мекенжайындағы 64 ш. м. кеңсе үй-жайын жалға беру.\n2. Жалдау ақысы айына 360 000 теңге.\n3. Шарттың мерзімі — 12 ай.`),
  d(`${ROOT_KK}\\Келісімшарттар\\Қызмет көрсету шарты.docx`, 'document', 67_100, 200,
    `ҚЫЗМЕТ КӨРСЕТУ ШАРТЫ № 3-Қ\nОрындаушы Тапсырыс берушіге бухгалтерлік қызметтерді көрсетуге міндеттенеді.\nҚызметтердің құны айына 150 000 теңге. Шарт 2024 жылғы 31 желтоқсанға дейін қолданылады.`),
  d(`${ROOT_KK}\\Хаттар\\Әкімдікке хат.docx`, 'document', 41_200, 15,
    `Алматы қаласы Бостандық аудандық әкімдігіне\n\n«Стройинвест» ЖШС Абай даңғылы, 150 мекенжайындағы ғимараттың аумағын абаттандыру жұмыстарын жүргізуге рұқсат беруіңізді сұрайды.\nҚосымша: жоспар-сызба, жалдау шартының көшірмесі.\nДиректор А. С. Ахметов`),
  d(`${ROOT_KK}\\Хаттар\\Шағым бойынша жауап.pdf`, 'pdf', 210_340, 45,
    `Құрметті Нұрлан мырза!\nСіздің 2024 жылғы 12 шілдедегі шағымыңызға жауап ретінде хабарлаймыз: кеңсе үй-жайындағы жылу жүйесінің ақаулығы 3 жұмыс күні ішінде жойылады. Жалдау ақысы бұл кезеңге қайта есептеледі.`),
  d(`${ROOT_KK}\\Презентациялар\\Компания таныстырылымы.pptx`, 'presentation', 7_800_120, 70,
    `«Стройинвест» ЖШС — 15 жыл нарықта\nБіздің қызметтер: коммерциялық жылжымайтын мүлікті жалға беру, құрылыс, жобалау.\n120-дан астам жалдау шарты, 46 қызметкер, 3 қалада өкілдік.`),
  d(`${ROOT_KK}\\Кесте\\Қызметкерлер тізімі.xlsx`, 'spreadsheet', 52_400, 5,
    `Аты-жөні	Лауазымы	Бөлім	Телефон\nАхметов Асқар	Директор	Басшылық	+7 701 000 00 01\nСериков Дәулет	Инженер-сметчик	Жобалау	+7 701 000 00 27\nНұрлан Айгерім	Жалдау менеджері	Жалдау	+7 701 000 00 14`),
  d(`${ROOT_KK}\\Жазбалар\\жиналыс.txt`, 'text', 2_100, 1,
    `Жиналыс — 7 қыркүйек\nКүн тәртібі:\n1. Жалдау шарты № 5 бойынша төлемдер.\n2. 2025 жылға арналған стратегия.\n3. Кеңсеге жаңа жиһаз сатып алу.`),
  d(`${ROOT_KK}\\Деректер\\клиенттер.csv`, 'data', 14_500, 8,
    `id,name,city,contract\n1,Береке ЖШС,Алматы,Жалдау шарты № 5\n2,Нұрлан ЖК,Алматы,Договор аренды № 12\n3,Казтранс ЖШС,Астана,Договор аренды № 7`),
  d(`${ROOT_KK}\\Веб\\index.html`, 'web', 6_800, 120,
    `<title>Стройинвест — кеңсе жалдау</title>\nАлматыдағы кеңсе үй-жайларын жалға беру. Ыңғайлы орналасу, икемді шарттар, тәулік бойы күзет.\nБайланыс: +7 727 000 00 00`),

  d(`${ROOT_EN}\\report-q3.pdf`, 'pdf', 1_240_330, 40,
    `Q3 2024 Management Report\n\nRevenue grew 14% quarter over quarter, driven by the commercial lease portfolio. Office rent expenses remained flat at KZT 450,000 per month under lease agreement No. 12.\nHeadcount: 48. Cash position: KZT 38.2M.\nOutlook: renew the office lease, sign two new warehouse lease agreements in Q4.`),
  d(`${ROOT_EN}\\lease-agreement-template.docx`, 'document', 92_400, 300,
    `COMMERCIAL LEASE AGREEMENT (TEMPLATE)\n\nThis Lease Agreement is entered into between the Landlord and the Tenant. The Landlord agrees to lease to the Tenant the premises described in Schedule A.\n1. Term. The initial term of this lease is twelve (12) months.\n2. Rent. Tenant shall pay monthly rent in advance on the first day of each month.\n3. Security deposit. An amount equal to one month's rent.`),
  d(`${ROOT_EN}\\Projects\\qidir\\README.md`, 'text', 8_120, 0.1,
    `# QIDIR\n\nLocal full-text search for your documents with first-class Russian, Kazakh and English support.\n\n## Features\n- Indexes DOCX, XLSX, PPTX, PDF, TXT, EPUB and more\n- Smart morphology-aware search and exact phrase search\n- Fast preview with highlighted matches\n\n## Building\n\`\`\`\nnpm install && npm run tauri dev\n\`\`\``),
  d(`${ROOT_EN}\\Projects\\qidir\\src\\search.rs`, 'code', 24_700, 0.5,
    `//! Query parsing and search execution.\nuse tantivy::query::QueryParser;\n\npub fn search(index: &Index, request: &SearchRequest) -> Result<SearchResponse> {\n    let parser = QueryParser::for_index(index, vec![name, content]);\n    let query = parser.parse_query(&request.query)?;\n    // TODO: lease facets by category\n    let top = searcher.search(&query, &TopDocs::with_limit(request.limit))?;\n    Ok(build_response(top))\n}`),
  d(`${ROOT_EN}\\Projects\\qidir\\src\\lib\\utils\\format.ts`, 'code', 3_100, 0.2,
    `export function formatSize(bytes: number, locale: string): string {\n  const units = ['B', 'KB', 'MB', 'GB'];\n  let i = 0;\n  while (bytes >= 1024 && i < units.length - 1) { bytes /= 1024; i++; }\n  return new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(bytes) + ' ' + units[i];\n}`),
  d(`${ROOT_EN}\\Projects\\qidir\\package.json`, 'data', 1_450, 0.2,
    `{\n  "name": "qidir-desktop",\n  "private": true,\n  "scripts": { "dev": "vite", "build": "vite build" }\n}`),
  d(`${ROOT_EN}\\Books\\Clean Architecture.epub`, 'ebook', 3_120_000, 500,
    `Chapter 5. Object-Oriented Programming\nThe basis of a good architecture is the understanding and application of the principles of object-oriented design. Dependency inversion lets high-level policies remain independent of low-level details.`),
  d(`${ROOT_EN}\\Mail\\Lease renewal.eml`, 'email', 12_300, 2,
    `From: landlord@stroyinvest.kz\nTo: aslan@example.com\nSubject: Lease renewal — office 312\n\nHi Aslan,\nAs discussed, we are happy to renew the lease for office 312 for another 12 months at the current rate. Please find the draft addendum attached.\nBest regards, Askar`),
  d(`${ROOT_EN}\\Downloads\\invoice-2024-118.pdf`, 'pdf', 88_400, 12,
    `INVOICE #118\nBill to: Nurlan IE\nDescription: Office rent, lease agreement No. 12, September 2024\nAmount due: KZT 470,000.00\nDue date: 05.09.2024`),
  d(`${ROOT_EN}\\Downloads\\setup-notes.txt`, 'text', 640, 6,
    `Install steps:\n1. Run installer\n2. Add folders to index\n3. Wait for indexing to finish\n4. Search!`, 'ASCII'),
  d(`${ROOT_EN}\\Downloads\\photo-archive.7z`, 'other', 512_000_000, 90, null),
  d(`${ROOT_EN}\\Spreadsheets\\budget-2025.xlsx`, 'spreadsheet', 210_300, 7,
    `Budget 2025\nLine item	Q1	Q2	Q3	Q4\nOffice lease	1,410,000	1,410,000	1,410,000	1,410,000\nSalaries	6,900,000	6,900,000	7,200,000	7,200,000\nMarketing	800,000	900,000	900,000	1,100,000`)
];

export const DEFAULT_EXCLUDE_GLOBS = [
  '**/node_modules/**', '**/.git/**', '**/target/**', '**/dist/**', '**/$RECYCLE.BIN/**',
  '**/System Volume Information/**', '**/AppData/Local/Temp/**', '**/*.tmp', '**/~$*'
];

export function defaultSettings(): Settings {
  return {
    version: 1,
    roots: [
      { path: ROOT_RU, enabled: true },
      { path: ROOT_KK, enabled: true },
      { path: ROOT_EN, enabled: true }
    ],
    excludeGlobs: [...DEFAULT_EXCLUDE_GLOBS],
    includeExtensions: [],
    excludeExtensions: ['log', 'tmp'],
    maxFileSizeBytes: 50 * 1024 * 1024,
    storeContent: true,
    indexHidden: false,
    followLinks: false,
    watchChanges: true,
    workerThreads: 0,
    writerMemoryMb: 256,
    pageSize: 50,
    uiLanguage: 'ru',
    theme: 'system',
    reindexOnStart: false,
    extractPdf: true
  };
}

// ---------- helpers ----------

export function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function fileName(path: string): string {
  const i = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
  return i >= 0 ? path.slice(i + 1) : path;
}

function extOf(name: string): string {
  const i = name.lastIndexOf('.');
  return i > 0 ? name.slice(i + 1).toLowerCase() : '';
}

/** Splits a query into plain terms, dropping operators and field prefixes. */
export function queryTerms(query: string, mode: SearchMode): string[] {
  const terms: string[] = [];
  const re = /"([^"]+)"|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(query)) !== null) {
    if (m[1] !== undefined) {
      if (mode === 'exact') terms.push(m[1].toLowerCase());
      else terms.push(...m[1].toLowerCase().split(/\s+/).filter(Boolean));
      continue;
    }
    let t = m[2];
    if (/^(or|and|not)$/i.test(t)) continue;
    if (t.startsWith('-')) continue;
    const colon = t.indexOf(':');
    if (colon > 0 && /^(name|ext|path|content)$/i.test(t.slice(0, colon))) t = t.slice(colon + 1);
    t = t.replace(/[*~]+$/g, '').toLowerCase();
    if (t) terms.push(t);
  }
  return terms;
}

/** Crude stemming for the mock's "smart" mode: strips a short inflectional tail from longer words. */
export function stem(term: string): string {
  if (term.length <= 4) return term;
  if (term.length <= 6) return term.slice(0, -1);
  return term.slice(0, -2);
}

function matchers(terms: string[], mode: SearchMode): RegExp[] {
  return terms.map((t) => {
    const base = mode === 'smart' ? stem(t) : t;
    const esc = base.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const tail = mode === 'smart' ? '[\\p{L}\\p{N}]{0,4}' : '';
    return new RegExp(`${esc}${tail}`, 'giu');
  });
}

function docMatches(doc: MockDoc, terms: string[], mode: SearchMode): boolean {
  if (terms.length === 0) return true;
  const hay = `${doc.path}\n${doc.content ?? ''}`.toLowerCase();
  const stems = terms.map((t) => (mode === 'smart' ? stem(t) : t));
  return stems.every((s) => hay.includes(s));
}

export function buildSnippet(content: string, res: RegExp[], maxChars: number): { html: string; count: number } {
  if (!content) return { html: '', count: 0 };
  let count = 0;
  let firstIdx = -1;
  for (const re of res) {
    re.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(content)) !== null) {
      count++;
      if (firstIdx < 0 || m.index < firstIdx) firstIdx = m.index;
      if (m[0].length === 0) re.lastIndex++;
    }
  }
  let start = 0;
  if (firstIdx > maxChars / 3) start = Math.max(0, firstIdx - Math.floor(maxChars / 3));
  // snap to word boundary
  if (start > 0) {
    const sp = content.indexOf(' ', start);
    if (sp >= 0 && sp - start < 30) start = sp + 1;
  }
  let piece = content.slice(start, start + maxChars).replace(/\s+/g, ' ').trim();
  let html = escapeHtml(piece);
  for (const re of res) {
    re.lastIndex = 0;
    html = html.replace(re, (m) => `<mark>${m}</mark>`);
  }
  if (start > 0) html = '…' + html;
  if (start + maxChars < content.length) html += '…';
  return { html, count };
}

function toHit(doc: MockDoc, res: RegExp[], snippetChars: number): SearchHit {
  const name = fileName(doc.path);
  const root = [ROOT_RU, ROOT_KK, ROOT_EN].find((r) => doc.path.startsWith(r)) ?? '';
  const { html, count } = buildSnippet(doc.content ?? '', res, snippetChars);
  const nameHits = res.reduce((n, re) => { re.lastIndex = 0; return n + (re.test(name) ? 3 : 0); }, 0);
  return {
    path: doc.path, name, ext: extOf(name), category: doc.category, root, size: doc.size, modified: doc.modified,
    score: Math.round((count + nameHits) * 10 + (doc.content ? 5 : 0)) / 10, hasContent: !!doc.content,
    snippet: html, encoding: doc.encoding
  };
}

function sortHits(hits: SearchHit[], sort: SearchRequest['sort']): SearchHit[] {
  const c = new Intl.Collator(undefined, { sensitivity: 'base', numeric: true });
  const by: Record<SearchRequest['sort'], (a: SearchHit, b: SearchHit) => number> = {
    relevance: (a, b) => b.score - a.score || b.modified - a.modified,
    modified_desc: (a, b) => b.modified - a.modified,
    modified_asc: (a, b) => a.modified - b.modified,
    size_desc: (a, b) => b.size - a.size,
    size_asc: (a, b) => a.size - b.size,
    name_asc: (a, b) => c.compare(a.name, b.name),
    name_desc: (a, b) => c.compare(b.name, a.name)
  };
  return [...hits].sort(by[sort]);
}

function facetOf(values: string[]): FacetCount[] {
  const m = new Map<string, number>();
  for (const v of values) if (v) m.set(v, (m.get(v) ?? 0) + 1);
  return [...m.entries()].map(([value, count]) => ({ value, count })).sort((a, b) => b.count - a.count || a.value.localeCompare(b.value));
}

export function interpret(query: string, mode: SearchMode): string {
  const terms = queryTerms(query, mode);
  if (terms.length === 0) return '';
  if (mode === 'exact') return terms.map((t) => `"${t}"`).join(' AND ');
  return terms.map((t) => `${stem(t)}*`).join(' AND ');
}

/** Pure search over a document set — used by the mock backend and by unit tests. */
export function mockSearch(docs: MockDoc[], request: SearchRequest): SearchResponse {
  const t0 = performance.now();
  const terms = queryTerms(request.query, request.mode);
  const res = matchers(terms, request.mode);
  let matched = docs.filter((doc) => docMatches(doc, terms, request.mode));
  if (request.roots.length) matched = matched.filter((doc) => request.roots.some((r) => doc.path.startsWith(r)));
  if (request.withContentOnly) matched = matched.filter((doc) => !!doc.content);
  if (request.sizeMin != null) matched = matched.filter((doc) => doc.size >= request.sizeMin!);
  if (request.sizeMax != null) matched = matched.filter((doc) => doc.size <= request.sizeMax!);
  if (request.modifiedFrom != null) matched = matched.filter((doc) => doc.modified >= request.modifiedFrom!);
  if (request.modifiedTo != null) matched = matched.filter((doc) => doc.modified <= request.modifiedTo!);

  // Facets are computed before category/extension narrowing so users can still see the other options.
  const allHits = matched.map((doc) => toHit(doc, res, request.snippetChars || 160));
  const facets = {
    categories: facetOf(allHits.map((h) => h.category)),
    extensions: facetOf(allHits.map((h) => h.ext)),
    roots: facetOf(allHits.map((h) => h.root))
  };
  let hits = allHits;
  if (request.categories.length) hits = hits.filter((h) => request.categories.includes(h.category));
  if (request.extensions.length) hits = hits.filter((h) => request.extensions.includes(h.ext));
  hits = sortHits(hits, request.sort);
  const total = hits.length;
  const page = hits.slice(request.offset, request.offset + request.limit);
  return {
    total, hits: page, facets, tookMs: Math.max(1, Math.round(performance.now() - t0)),
    interpretation: interpret(request.query, request.mode), offset: request.offset, limit: request.limit
  };
}

export function buildPreview(doc: MockDoc, query: string, mode: SearchMode): Preview {
  const content = doc.content ?? '';
  const res = matchers(queryTerms(query, mode), mode);
  const segments: PreviewSegment[] = [];
  const marks: Array<[number, number]> = [];
  for (const re of res) {
    re.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(content)) !== null) {
      if (m[0].length === 0) { re.lastIndex++; continue; }
      marks.push([m.index, m.index + m[0].length]);
    }
  }
  marks.sort((a, b) => a[0] - b[0]);
  let pos = 0;
  let total = 0;
  for (const [s, e] of marks) {
    if (s < pos) continue;
    if (s > pos) segments.push({ text: content.slice(pos, s), highlight: false });
    segments.push({ text: content.slice(s, e), highlight: true });
    total++;
    pos = e;
  }
  if (pos < content.length) segments.push({ text: content.slice(pos), highlight: false });
  return { path: doc.path, segments, totalMatches: total, truncated: false, source: 'index', encoding: doc.encoding, chars: content.length };
}

// ---------- backend ----------

function urlParam(name: string): string | null {
  try { return new URLSearchParams(window.location.search).get(name); } catch { return null; }
}

export function createMockBackend(): Backend {
  const docs = [...SAMPLE_DOCS];
  const settings = defaultSettings();
  const lang = urlParam('lang');
  if (lang === 'ru' || lang === 'kk' || lang === 'en') settings.uiLanguage = lang as UiLanguage;
  const theme = urlParam('theme');
  if (theme === 'dark' || theme === 'light' || theme === 'system') settings.theme = theme as Theme;
  if (urlParam('empty') === '1') settings.roots = [];

  let indexedDocs = settings.roots.length ? docs.length : 0;
  let lastIndexed: number | null = settings.roots.length ? NOW - 5 * 60 : null;
  let progress: IndexProgress = {
    running: false, phase: 'idle', scanned: 0, queued: 0, indexed: 0, unchanged: 0, nameOnly: 0, failed: 0,
    deleted: 0, bytes: 0, current: null, error: null, elapsedMs: 0
  };
  const progressListeners = new Set<(p: IndexProgress) => void>();
  const finishedListeners = new Set<(p: IndexProgress) => void>();
  let timer: ReturnType<typeof setInterval> | null = null;

  function emitProgress() { for (const cb of progressListeners) cb({ ...progress }); }
  function finish(phase: IndexProgress['phase']) {
    if (timer) clearInterval(timer);
    timer = null;
    progress = { ...progress, running: false, phase, current: null };
    if (phase === 'done') { indexedDocs = docs.length; lastIndexed = Math.floor(Date.now() / 1000); }
    emitProgress();
    for (const cb of finishedListeners) cb({ ...progress });
  }

  function startIndexing(full: boolean): Promise<void> {
    if (progress.running) return Promise.reject('Indexing is already running');
    const totalFiles = full ? 3210 : 1180;
    const start = Date.now();
    progress = {
      running: true, phase: 'scanning', scanned: 0, queued: 0, indexed: 0, unchanged: 0, nameOnly: 0, failed: 0,
      deleted: 0, bytes: 0, current: settings.roots[0]?.path ?? null, error: null, elapsedMs: 0
    };
    emitProgress();
    const durationMs = 6000;
    timer = setInterval(() => {
      const elapsed = Date.now() - start;
      const f = Math.min(1, elapsed / durationMs);
      progress.elapsedMs = elapsed;
      if (f < 0.3) {
        progress.phase = 'scanning';
        progress.scanned = Math.round(totalFiles * (f / 0.3));
        progress.current = docs[Math.floor(Math.random() * docs.length)].path;
      } else if (f < 0.85) {
        const g = (f - 0.3) / 0.55;
        progress.phase = 'extracting';
        progress.scanned = totalFiles;
        progress.queued = totalFiles;
        progress.indexed = Math.round(totalFiles * g);
        progress.unchanged = full ? 0 : Math.round(totalFiles * 0.6 * g);
        progress.nameOnly = Math.round(progress.indexed * 0.08);
        progress.failed = Math.round(progress.indexed * 0.004);
        progress.bytes = Math.round(progress.indexed * 240_000);
        progress.current = docs[Math.floor(g * (docs.length - 1))].path;
      } else if (f < 0.93) {
        progress.phase = 'cleaning';
        progress.indexed = totalFiles;
        progress.deleted = 12;
        progress.current = null;
      } else if (f < 1) {
        progress.phase = 'committing';
        progress.current = null;
      } else {
        finish('done');
        return;
      }
      emitProgress();
    }, 250);
    return Promise.resolve();
  }

  const delay = <T,>(v: T, ms = 60): Promise<T> => new Promise((r) => setTimeout(() => r(v), ms));

  const backend: Backend = {
    getSettings: () => delay(structuredClone(settings), 10),
    saveSettings: (s) => { Object.assign(settings, structuredClone(s)); return delay(structuredClone(settings), 20); },
    listDrives: () => delay<DriveInfo[]>([
      { path: 'C:\\', label: 'Windows (C:)' }, { path: 'D:\\', label: 'Данные (D:)' }, { path: 'E:\\', label: 'USB (E:)' }
    ]),
    pickFolders: () => delay(['D:\\Проекты'], 200),
    startIndexing,
    cancelIndexing: () => { if (progress.running) finish('cancelled'); return Promise.resolve(); },
    getProgress: () => Promise.resolve({ ...progress }),
    search: (request) => {
      if (indexedDocs === 0) {
        const empty = mockSearch([], request);
        return delay(empty, 30);
      }
      return delay(mockSearch(docs, request), 40 + Math.random() * 60);
    },
    getPreview: (path, query, mode) => {
      const doc = docs.find((x) => x.path === path);
      if (!doc) return Promise.reject(`File not found in index: ${path}`);
      return delay(buildPreview(doc, query, mode), 120);
    },
    getStats: () => delay<IndexStats>({
      documents: indexedDocs ? 12_480 : 0,
      withContent: indexedDocs ? 11_902 : 0,
      indexSizeBytes: indexedDocs ? 1_284_000_000 : 0,
      lastIndexed,
      roots: settings.roots.map((r) => ({ path: r.path, enabled: r.enabled, exists: true, documents: indexedDocs ? Math.round(12_480 / settings.roots.length) : 0 })),
      indexing: progress.running,
      watching: settings.watchChanges && indexedDocs > 0,
      schemaVersion: 3,
      dataDir: 'C:\\Users\\Aslan\\AppData\\Roaming\\QIDIR'
    }),
    clearIndex: () => { indexedDocs = 0; lastIndexed = null; return delay(undefined, 100); },
    openFile: (path) => { console.info('[mock] open', path); return Promise.resolve(); },
    revealInExplorer: (path) => { console.info('[mock] reveal', path); return Promise.resolve(); },
    openWith: (path) => { console.info('[mock] open with', path); return Promise.resolve(); },
    copyText: async (text) => {
      try { await navigator.clipboard.writeText(text); } catch { /* clipboard may be unavailable in headless */ }
    },
    analyzeText: (text, mode) => Promise.resolve(queryTerms(text, mode).map((t) => (mode === 'smart' ? stem(t) : t))),
    getAppInfo: () => delay<AppInfo>({ version: '0.1.0-mock', dataDir: 'C:\\Users\\Aslan\\AppData\\Roaming\\QIDIR', logDir: 'C:\\Users\\Aslan\\AppData\\Roaming\\QIDIR\\logs', platform: 'browser-mock' }),
    openLogsFolder: () => Promise.resolve(),
    openDataFolder: () => Promise.resolve(),
    onIndexProgress: (cb) => { progressListeners.add(cb); return () => progressListeners.delete(cb); },
    onIndexFinished: (cb) => { finishedListeners.add(cb); return () => finishedListeners.delete(cb); }
  };

  if (urlParam('indexing') === '1') setTimeout(() => { void startIndexing(true); }, 300);
  return backend;
}
