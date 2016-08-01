var searchIndex = {};
searchIndex["wkhtmltopdf"] = {"doc":"","items":[[3,"PdfOutput","wkhtmltopdf","Generated PDF output",null,null],[3,"Margin","","PDF Margins",null,null],[12,"top","","",0,null],[12,"bottom","","",0,null],[12,"left","","",0,null],[12,"right","","",0,null],[3,"PdfApplication","","Structure for initializing the underlying wkhtmltopdf",null,null],[3,"PdfBuilder","","High-level builder for generating PDFs (initialized from `PdfApplication`)",null,null],[4,"Error","","The error type for wkhtmltopdf generation",null,null],[13,"IoError","","Indicates an I/O error that occurred during PDF generation",1,null],[13,"IllegalInit","","Indicates the wkhtmltopdf could be be initialized because it can only be initialized once per process (wkhtmltopdf limitation)",1,null],[13,"NotInitialized","","Indicates that wkhtmltopdf has not yet been initialized in this process",1,null],[13,"Blocked","","Indicates that wkhtmltopdf is blocked by another request within this process (wkhtmltopdf limitation)",1,null],[13,"ThreadMismatch","","Indicates that wkhtmltopdf was initialized on a different thread than this PDF generation atttempt (wkhtmltopdf limitation)",1,null],[13,"ConversionFailed","","Indicates that wkhtmltopdf conversion failed - internal error message comes directly from wkhtmltopdf",1,null],[13,"GlobalSettingFailure","","Indicates that wkhtmltopdf failed to set a particular global setting",1,null],[13,"ObjectSettingFailure","","Indicates that wkhtmltopdf failed to set a particular object setting",1,null],[4,"PageSize","","Physical size of the paper",null,null],[13,"A1","","",2,null],[13,"A2","","",2,null],[13,"A3","","",2,null],[13,"A4","","",2,null],[13,"A5","","",2,null],[13,"A6","","",2,null],[13,"A7","","",2,null],[13,"A8","","",2,null],[13,"A9","","",2,null],[13,"B0","","",2,null],[13,"B1","","",2,null],[13,"B2","","",2,null],[13,"B3","","",2,null],[13,"B4","","",2,null],[13,"B5","","",2,null],[13,"B6","","",2,null],[13,"B7","","",2,null],[13,"B8","","",2,null],[13,"B9","","",2,null],[13,"B10","","",2,null],[13,"C5E","","",2,null],[13,"Comm10E","","",2,null],[13,"DLE","","",2,null],[13,"Executive","","",2,null],[13,"Folio","","",2,null],[13,"Ledger","","",2,null],[13,"Legal","","",2,null],[13,"Letter","","",2,null],[13,"Tabloid","","",2,null],[13,"Custom","","Custom paper size: (width, height)",2,null],[4,"Size","","Unit-aware sizes",null,null],[13,"Millimeters","","",3,null],[13,"Inches","","",3,null],[4,"Orientation","","PDF Orientation",null,null],[13,"Landscape","","",4,null],[13,"Portrait","","",4,null],[0,"lowlevel","","Low-level wkhtmltopdf without the raw pointers",null,null],[3,"PdfGuard","wkhtmltopdf::lowlevel","Handles initialization and deinitialization of wkhtmltopdf",null,null],[3,"PdfGlobalSettings","","Safe wrapper for managing wkhtmltopdf global settings",null,null],[3,"PdfObjectSettings","","Safe wrapper for managing wkhtmltopdf object settings",null,null],[3,"PdfConverter","","Safe wrapper for working with the wkhtmltopdf converter",null,null],[5,"pdf_init","","Initializes wkhtmltopdf",null,{"inputs":[],"output":{"name":"result"}}],[11,"new","","Instantiate PdfGlobalSettings",5,{"inputs":[],"output":{"name":"result"}}],[11,"set","","",5,null],[11,"create_converter","","",5,null],[11,"add_page_object","","",6,null],[11,"add_html_object","","",6,null],[11,"convert","","",6,null],[11,"new","","",7,{"inputs":[],"output":{"name":"pdfobjectsettings"}}],[11,"set","","",7,null],[11,"drop","","",5,null],[11,"drop","","",6,null],[11,"drop","","",7,null],[11,"drop","wkhtmltopdf","",8,null],[11,"drop","wkhtmltopdf::lowlevel","",9,null],[11,"fmt","wkhtmltopdf","",1,null],[11,"fmt","","",1,null],[11,"description","","",1,null],[11,"cause","","",1,null],[11,"from","","",1,{"inputs":[{"name":"error"}],"output":{"name":"error"}}],[6,"Result","","A specialized `Result` type for wkhtmltopdf generation",null,null],[11,"fmt","","",2,null],[11,"clone","","",3,null],[11,"fmt","","",3,null],[11,"fmt","","",4,null],[11,"fmt","","",0,null],[11,"from","","Performs the conversion using `size` for all margins",0,{"inputs":[{"name":"size"}],"output":{"name":"margin"}}],[11,"from","","Performs the converstion to margins from an ordered tuple representing: (top &amp; bottom, left &amp; right)",0,null],[11,"from","","Performs the converstion to margins from an ordered tuple representing: (top, left &amp; right, bottom)",0,null],[11,"from","","Performs the converstion to margins from an ordered tuple representing: (top, right, bottom, left)",0,null],[11,"new","","",10,{"inputs":[],"output":{"name":"result"}}],[11,"builder","","",10,null],[11,"clone","","",11,null],[11,"page_size","","The paper size of the output document (default A4)",11,null],[11,"margin","","Size of the page margins (default 10mm on all sides)",11,null],[11,"orientation","","The orientation of the output document (default portrait)",11,null],[11,"dpi","","What dpi should we use when printin (default 72)",11,null],[11,"image_quality","","JPEG image compression quality in percentage (default 94)",11,null],[11,"title","","Title of the output document (default none)",11,null],[11,"outline","","Enabled generating an outline (table of contents) in the sidebar with a specified depth",11,null],[11,"global_setting","","Set a global setting not explicitly supported by the PdfBuilder",11,null],[11,"object_setting","","Set an object setting not explicitly supported by the PdfBuilder",11,null],[11,"build_from_url","","Build a PDF using a URL as the source input",11,null],[11,"build_from_path","","Build a PDF using the provided HTML from a local file",11,null],[11,"build_from_html","","Build a PDF using the provided HTML source input",11,null],[11,"global_settings","","Use the relevant settings to construct a low-level instance of `PdfGlobalSettings`",11,null],[11,"object_settings","","Use the relevant settings to construct a low-level instance of `PdfObjectSettings`",11,null],[11,"save","","",8,null],[11,"read","","",8,null],[11,"fmt","","",8,null]],"paths":[[3,"Margin"],[4,"Error"],[4,"PageSize"],[4,"Size"],[4,"Orientation"],[3,"PdfGlobalSettings"],[3,"PdfConverter"],[3,"PdfObjectSettings"],[3,"PdfOutput"],[3,"PdfGuard"],[3,"PdfApplication"],[3,"PdfBuilder"]]};
initSearch(searchIndex);