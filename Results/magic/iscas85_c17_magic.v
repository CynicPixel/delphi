// NOR_NOT mapped module module_name

module module_name (
  input  ip_1,
  input  ip_2,
  input  ip_3,
  input  ip_4,
  input  ip_5,
  output op_1,
  output op_2
);

  wire wr_3;
  wire wr_4;
  wire wr_5;
  wire wr_6;
  wire wr_7;
  wire wr_8;
  wire wr_9;
  wire wr_10;
  wire wr_11;
  wire wr_12;
  wire wr_13;

  not    g1   ( wr_4     ,           ip_3     );
  not    g2   ( wr_7     ,           ip_4     );
  not    g3   ( wr_3     ,           ip_1     );
  not    g4   ( wr_6     ,           ip_2     );
  not    g5   ( wr_11    ,           ip_5     );
  nor    g6   ( wr_8     , wr_7     , wr_4     );
  nor    g7   ( wr_5     , wr_4     , wr_3     );
  nor    g8   ( wr_9     , wr_8     , wr_6     );
  nor    g9   ( wr_12    , wr_8     , wr_11    );
  nor    g10  ( wr_10    , wr_9     , wr_5     );
  nor    g11  ( wr_13    , wr_12    , wr_9     );
  not    g12  ( wr_1     ,           wr_10    );
  not    g13  ( wr_2     ,           wr_13    );

endmodule
